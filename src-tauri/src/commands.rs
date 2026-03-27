use std::path::PathBuf;
use tauri::ipc::Channel;

use crate::db;
use crate::models::*;
use crate::parser;
use crate::pypi;
use crate::scanner;
use crate::AppState;

#[tauri::command]
pub fn get_scan_status(state: tauri::State<'_, AppState>) -> Result<ScanStatus, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_scan_status(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_all_venvs(state: tauri::State<'_, AppState>) -> Result<Vec<Venv>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_all_venvs(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_venv_packages(
    state: tauri::State<'_, AppState>,
    venv_id: i64,
) -> Result<Vec<Package>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_venv_packages(&conn, venv_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_packages(
    state: tauri::State<'_, AppState>,
    query: String,
) -> Result<Vec<PackageSearchResult>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::search_packages(&conn, &query).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_venvs_with_package(
    state: tauri::State<'_, AppState>,
    package_name: String,
) -> Result<Vec<PackageSearchResult>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_venvs_with_package(&conn, &package_name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_package_dependencies(
    state: tauri::State<'_, AppState>,
    package_id: i64,
) -> Result<Vec<Dependency>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_package_dependencies(&conn, package_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn scan_venvs(
    state: tauri::State<'_, AppState>,
    root_path: Option<String>,
    on_progress: Channel<ScanProgress>,
) -> Result<usize, String> {
    let root = root_path
        .map(PathBuf::from)
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")));

    // Phase 1: find all venvs
    let _ = on_progress.send(ScanProgress {
        total_found: 0,
        current: 0,
        current_path: "Scanning filesystem...".to_string(),
        phase: "scanning".to_string(),
    });

    let venv_paths = scanner::find_venvs(&root);
    let total = venv_paths.len();

    // Phase 2: parse and store
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::clear_all(&conn).map_err(|e| e.to_string())?;

        for (i, venv_path) in venv_paths.iter().enumerate() {
            let _ = on_progress.send(ScanProgress {
                total_found: total,
                current: i + 1,
                current_path: venv_path.to_string_lossy().to_string(),
                phase: "indexing".to_string(),
            });

            let cfg = match parser::parse_pyvenv_cfg(venv_path) {
                Some(c) => c,
                None => continue,
            };

            let packages = parser::parse_packages(venv_path);

            if let Err(e) = db::insert_venv_full(&conn, venv_path, &cfg, &packages) {
                eprintln!("Failed to index {}: {}", venv_path.display(), e);
            }
        }
    }

    Ok(total)
}

#[tauri::command]
pub async fn check_outdated(
    state: tauri::State<'_, AppState>,
    venv_id: i64,
) -> Result<Vec<PypiVersionInfo>, String> {
    let packages = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::get_venv_packages(&conn, venv_id).map_err(|e| e.to_string())?
    };

    let package_list: Vec<(String, String)> = packages
        .into_iter()
        .map(|p| (p.name, p.version))
        .collect();

    let results = pypi::check_packages(package_list, &state.pypi_cache).await;
    Ok(results)
}

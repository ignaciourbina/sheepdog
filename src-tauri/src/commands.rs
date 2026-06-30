use tauri::ipc::Channel;

use crate::db;
use crate::models::*;
use crate::pypi;
use crate::service;
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
pub fn get_export_rows(state: tauri::State<'_, AppState>) -> Result<Vec<ExportRow>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_export_rows(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn scan_venvs(
    state: tauri::State<'_, AppState>,
    root_path: Option<String>,
    on_progress: Channel<ScanProgress>,
) -> Result<usize, String> {
    let root = root_path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/")));

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let summary = service::scan_and_index(&conn, &root, |progress| {
        let _ = on_progress.send(progress);
    })?;

    Ok(summary.discovered)
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

    let package_list: Vec<(String, String)> =
        packages.into_iter().map(|p| (p.name, p.version)).collect();

    let results = pypi::check_packages(package_list, &state.pypi_cache).await;
    Ok(results)
}

#[tauri::command]
pub fn read_project_file(project_path: String, filename: String) -> Result<String, String> {
    // Security: only allow known config file names
    const ALLOWED: &[&str] = &[
        "requirements.txt",
        "requirements-dev.txt",
        "requirements_dev.txt",
        "pyproject.toml",
        "setup.py",
        "setup.cfg",
        "Pipfile",
        "environment.yml",
    ];

    if !ALLOWED.contains(&filename.as_str()) {
        return Err("File not allowed".to_string());
    }

    let path = std::path::Path::new(&project_path).join(&filename);
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read {}: {}", filename, e))
}

#[tauri::command]
pub fn open_in_vscode(path: String) -> Result<(), String> {
    std::process::Command::new("/snap/bin/code")
        .arg("--new-window")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("Failed to open VS Code: {}", e))?;
    Ok(())
}

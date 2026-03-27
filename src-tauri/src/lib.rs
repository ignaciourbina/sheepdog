mod commands;
mod db;
mod models;
mod parser;
mod pypi;
mod scanner;

use std::path::PathBuf;
use std::sync::Mutex;

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub pypi_cache: pypi::PypiCache,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db_path = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("sheepdog")
        .join("sheepdog.db");

    std::fs::create_dir_all(db_path.parent().unwrap()).ok();
    let conn = rusqlite::Connection::open(&db_path).expect("Failed to open database");
    db::init_db(&conn).expect("Failed to initialize database");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            db: Mutex::new(conn),
            pypi_cache: pypi::PypiCache::new(),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_scan_status,
            commands::get_all_venvs,
            commands::get_venv_packages,
            commands::search_packages,
            commands::get_venvs_with_package,
            commands::get_package_dependencies,
            commands::scan_venvs,
            commands::check_outdated,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

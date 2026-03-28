mod commands;
mod db;
mod demo;
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

pub fn run_with_args(args: &[String]) {
    let is_demo = args.iter().any(|a| a == "--demo");

    let conn = if is_demo {
        let conn = rusqlite::Connection::open_in_memory()
            .expect("Failed to open in-memory database");
        db::init_db(&conn).expect("Failed to initialize database");
        demo::populate_demo_data(&conn).expect("Failed to populate demo data");
        eprintln!("Sheepdog: running in demo mode (in-memory DB, fake data)");
        conn
    } else {
        let db_path = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("sheepdog")
            .join("sheepdog.db");
        std::fs::create_dir_all(db_path.parent().unwrap()).ok();
        let conn = rusqlite::Connection::open(&db_path).expect("Failed to open database");
        db::init_db(&conn).expect("Failed to initialize database");
        conn
    };

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
            commands::read_project_file,
            commands::open_in_vscode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    run_with_args(&[]);
}

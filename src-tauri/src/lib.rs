mod cli;
mod commands;
mod db;
mod demo;
mod disk_usage;
mod export;
mod models;
mod parser;
mod pypi;
mod scanner;
mod service;

use std::sync::Mutex;

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub pypi_cache: pypi::PypiCache,
}

pub fn run_from_args(args: &[String]) -> i32 {
    match cli::parse_startup_from(args) {
        Ok(cli::StartupMode::Gui { demo }) => {
            run_gui(demo);
            0
        }
        Ok(cli::StartupMode::Cli(cli_args)) => match cli::run_cli(cli_args) {
            Ok(()) => 0,
            Err(error) => {
                eprintln!("{}", error.message());
                error.code()
            }
        },
        Err(error) => {
            let code = error.exit_code();
            let _ = error.print();
            code
        }
    }
}

pub fn run_gui(demo: bool) {
    let conn = if demo {
        let conn = service::open_demo_db().expect("Failed to initialize demo database");
        eprintln!("Sheepdog: running in demo mode (in-memory DB, fake data)");
        conn
    } else {
        service::open_cache_db().expect("Failed to initialize database")
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
            commands::get_export_rows,
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
    run_gui(false);
}

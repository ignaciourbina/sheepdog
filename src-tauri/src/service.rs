use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;
use serde::Serialize;

use crate::db;
use crate::demo;
use crate::models::ScanProgress;
use crate::parser;
use crate::scanner;

#[derive(Debug, Clone, Serialize)]
pub struct ScanSummary {
    pub root_path: String,
    pub discovered: usize,
    pub indexed: usize,
}

pub fn cache_db_path() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("sheepdog")
        .join("sheepdog.db")
}

pub fn open_cache_db() -> rusqlite::Result<Connection> {
    let db_path = cache_db_path();
    std::fs::create_dir_all(db_path.parent().unwrap()).ok();
    let conn = Connection::open(&db_path)?;
    conn.busy_timeout(Duration::from_secs(3))?;
    db::init_db(&conn)?;
    Ok(conn)
}

pub fn open_demo_db() -> rusqlite::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    db::init_db(&conn)?;
    demo::populate_demo_data(&conn)?;
    Ok(conn)
}

pub fn scan_and_index<F>(
    conn: &Connection,
    root: &Path,
    mut on_progress: F,
) -> Result<ScanSummary, String>
where
    F: FnMut(ScanProgress),
{
    on_progress(ScanProgress {
        total_found: 0,
        current: 0,
        current_path: "Scanning filesystem...".to_string(),
        phase: "scanning".to_string(),
    });

    let venv_paths = scanner::find_venvs(root);
    let total = venv_paths.len();
    let mut indexed = 0;

    db::clear_all(conn).map_err(|e| e.to_string())?;

    for (i, venv_path) in venv_paths.iter().enumerate() {
        on_progress(ScanProgress {
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

        if let Err(e) = db::insert_venv_full(conn, venv_path, &cfg, &packages) {
            eprintln!("Failed to index {}: {}", venv_path.display(), e);
        } else {
            indexed += 1;
        }
    }

    Ok(ScanSummary {
        root_path: root.to_string_lossy().to_string(),
        discovered: total,
        indexed,
    })
}

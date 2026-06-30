use rusqlite::{params, Connection, Result};

use crate::disk_usage;
use crate::models::*;
use crate::parser;

/// Create all tables and indexes if they don't exist.
pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS venvs (
            id                INTEGER PRIMARY KEY AUTOINCREMENT,
            path              TEXT NOT NULL UNIQUE,
            project_path      TEXT NOT NULL,
            python_version    TEXT NOT NULL,
            python_executable TEXT,
            venv_name         TEXT NOT NULL,
            last_modified     TEXT,
            scanned_at        TEXT NOT NULL,
            size_bytes        INTEGER NOT NULL DEFAULT 0,
            config_files      TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS packages (
            id       INTEGER PRIMARY KEY AUTOINCREMENT,
            venv_id  INTEGER NOT NULL REFERENCES venvs(id) ON DELETE CASCADE,
            name     TEXT NOT NULL,
            version  TEXT NOT NULL,
            summary  TEXT,
            UNIQUE(venv_id, name)
        );

        CREATE TABLE IF NOT EXISTS dependencies (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            package_id   INTEGER NOT NULL REFERENCES packages(id) ON DELETE CASCADE,
            requires_raw TEXT NOT NULL,
            dep_name     TEXT NOT NULL,
            version_spec TEXT,
            extra        TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_packages_name ON packages(name);
        CREATE INDEX IF NOT EXISTS idx_packages_venv_id ON packages(venv_id);
        CREATE INDEX IF NOT EXISTS idx_deps_package_id ON dependencies(package_id);
        CREATE INDEX IF NOT EXISTS idx_deps_dep_name ON dependencies(dep_name);
        CREATE INDEX IF NOT EXISTS idx_venvs_project_path ON venvs(project_path);

        PRAGMA foreign_keys = ON;
        ",
    )?;

    // Migration: add config_files column if it doesn't exist (for existing DBs)
    let has_config_files: bool = conn
        .prepare("SELECT config_files FROM venvs LIMIT 0")
        .is_ok();
    if !has_config_files {
        conn.execute_batch("ALTER TABLE venvs ADD COLUMN config_files TEXT NOT NULL DEFAULT ''")?;
    }

    // Existing caches predate disk accounting. They remain readable with a
    // zero value and receive measured sizes during the next scan.
    let has_size_bytes: bool = conn.prepare("SELECT size_bytes FROM venvs LIMIT 0").is_ok();
    if !has_size_bytes {
        conn.execute_batch("ALTER TABLE venvs ADD COLUMN size_bytes INTEGER NOT NULL DEFAULT 0")?;
    }

    Ok(())
}

/// Clear all data before a fresh scan.
pub fn clear_all(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        DELETE FROM dependencies;
        DELETE FROM packages;
        DELETE FROM venvs;
        ",
    )?;
    Ok(())
}

/// Insert a venv and return its id.
pub fn insert_venv(
    conn: &Connection,
    path: &str,
    project_path: &str,
    python_version: &str,
    python_executable: &str,
    venv_name: &str,
    last_modified: &str,
    scanned_at: &str,
    size_bytes: i64,
    config_files: &str,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO venvs (path, project_path, python_version, python_executable, venv_name, last_modified, scanned_at, size_bytes, config_files)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![path, project_path, python_version, python_executable, venv_name, last_modified, scanned_at, size_bytes, config_files],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Insert a package and return its id.
pub fn insert_package(
    conn: &Connection,
    venv_id: i64,
    name: &str,
    version: &str,
    summary: Option<&str>,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO packages (venv_id, name, version, summary) VALUES (?1, ?2, ?3, ?4)",
        params![venv_id, name, version, summary],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Insert a dependency.
pub fn insert_dependency(
    conn: &Connection,
    package_id: i64,
    requires_raw: &str,
    dep_name: &str,
    version_spec: Option<&str>,
    extra: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO dependencies (package_id, requires_raw, dep_name, version_spec, extra)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![package_id, requires_raw, dep_name, version_spec, extra],
    )?;
    Ok(())
}

/// Insert a full venv with all its packages and dependencies in a single transaction.
pub fn insert_venv_full(
    conn: &Connection,
    venv_path: &std::path::Path,
    cfg: &parser::PyvenvConfig,
    packages: &[parser::PackageInfo],
) -> Result<i64> {
    let venv_name = venv_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let project_path = venv_path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let last_modified = std::fs::metadata(venv_path)
        .and_then(|m| m.modified())
        .ok()
        .map(|t| {
            let dt: chrono::DateTime<chrono::Local> = t.into();
            dt.format("%Y-%m-%d %H:%M").to_string()
        })
        .unwrap_or_default();

    let scanned_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let size_bytes = disk_usage::directory_size_bytes(venv_path) as i64;

    let config_files = venv_path
        .parent()
        .map(|p| parser::detect_config_files(p).join(","))
        .unwrap_or_default();

    let venv_id = insert_venv(
        conn,
        &venv_path.to_string_lossy(),
        &project_path,
        &cfg.version,
        &cfg.executable,
        &venv_name,
        &last_modified,
        &scanned_at,
        size_bytes,
        &config_files,
    )?;

    for pkg in packages {
        let pkg_id = insert_package(
            conn,
            venv_id,
            &pkg.name,
            &pkg.version,
            pkg.summary.as_deref(),
        )?;

        for req_raw in &pkg.requires_dist {
            let parsed = parser::parse_requires_dist(req_raw);
            insert_dependency(
                conn,
                pkg_id,
                req_raw,
                &parsed.name,
                parsed.version_spec.as_deref(),
                parsed.extra.as_deref(),
            )?;
        }
    }

    Ok(venv_id)
}

// ── Query functions ──────────────────────────────────────────────────────────

/// Get all venvs with their package counts.
pub fn get_all_venvs(conn: &Connection) -> Result<Vec<Venv>> {
    let mut stmt = conn.prepare(
        "SELECT v.id, v.path, v.project_path, v.python_version, v.python_executable,
                v.venv_name, v.last_modified, v.scanned_at,
                (SELECT COUNT(*) FROM packages p WHERE p.venv_id = v.id) as pkg_count,
                v.size_bytes, v.config_files
         FROM venvs v
         ORDER BY v.project_path",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(Venv {
            id: row.get(0)?,
            path: row.get(1)?,
            project_path: row.get(2)?,
            python_version: row.get(3)?,
            python_executable: row.get(4)?,
            venv_name: row.get(5)?,
            last_modified: row.get(6)?,
            scanned_at: row.get(7)?,
            package_count: row.get(8)?,
            size_bytes: row.get(9)?,
            config_files: row.get(10)?,
        })
    })?;

    rows.collect()
}

/// Get all packages for a specific venv.
pub fn get_venv_packages(conn: &Connection, venv_id: i64) -> Result<Vec<Package>> {
    let mut stmt = conn.prepare(
        "SELECT id, venv_id, name, version, summary FROM packages WHERE venv_id = ?1 ORDER BY name",
    )?;

    let rows = stmt.query_map(params![venv_id], |row| {
        Ok(Package {
            id: row.get(0)?,
            venv_id: row.get(1)?,
            name: row.get(2)?,
            version: row.get(3)?,
            summary: row.get(4)?,
        })
    })?;

    rows.collect()
}

/// Search packages by name across all venvs (case-insensitive LIKE).
pub fn search_packages(conn: &Connection, query: &str) -> Result<Vec<PackageSearchResult>> {
    let pattern = format!("%{}%", query);
    let mut stmt = conn.prepare(
        "SELECT p.name, p.version, v.path, v.project_path, v.python_version, v.venv_name, v.id, p.id
         FROM packages p
         JOIN venvs v ON p.venv_id = v.id
         WHERE p.name LIKE ?1 COLLATE NOCASE
         ORDER BY p.name, v.project_path",
    )?;

    let rows = stmt.query_map(params![pattern], |row| {
        Ok(PackageSearchResult {
            package_name: row.get(0)?,
            package_version: row.get(1)?,
            venv_path: row.get(2)?,
            project_path: row.get(3)?,
            python_version: row.get(4)?,
            venv_name: row.get(5)?,
            venv_id: row.get(6)?,
            package_id: row.get(7)?,
        })
    })?;

    rows.collect()
}

/// Get all venvs that contain a specific package (exact name match, case-insensitive).
pub fn get_venvs_with_package(
    conn: &Connection,
    package_name: &str,
) -> Result<Vec<PackageSearchResult>> {
    let mut stmt = conn.prepare(
        "SELECT p.name, p.version, v.path, v.project_path, v.python_version, v.venv_name, v.id, p.id
         FROM packages p
         JOIN venvs v ON p.venv_id = v.id
         WHERE p.name = ?1 COLLATE NOCASE
         ORDER BY v.project_path",
    )?;

    let rows = stmt.query_map(params![package_name], |row| {
        Ok(PackageSearchResult {
            package_name: row.get(0)?,
            package_version: row.get(1)?,
            venv_path: row.get(2)?,
            project_path: row.get(3)?,
            python_version: row.get(4)?,
            venv_name: row.get(5)?,
            venv_id: row.get(6)?,
            package_id: row.get(7)?,
        })
    })?;

    rows.collect()
}

/// Get dependencies for a specific package.
pub fn get_package_dependencies(conn: &Connection, package_id: i64) -> Result<Vec<Dependency>> {
    let mut stmt = conn.prepare(
        "SELECT id, package_id, requires_raw, dep_name, version_spec, extra
         FROM dependencies
         WHERE package_id = ?1
         ORDER BY dep_name",
    )?;

    let rows = stmt.query_map(params![package_id], |row| {
        Ok(Dependency {
            id: row.get(0)?,
            package_id: row.get(1)?,
            requires_raw: row.get(2)?,
            dep_name: row.get(3)?,
            version_spec: row.get(4)?,
            extra: row.get(5)?,
        })
    })?;

    rows.collect()
}

/// Get overall scan status.
pub fn get_scan_status(conn: &Connection) -> Result<ScanStatus> {
    let venv_count: i64 = conn.query_row("SELECT COUNT(*) FROM venvs", [], |r| r.get(0))?;
    let package_count: i64 = conn.query_row("SELECT COUNT(*) FROM packages", [], |r| r.get(0))?;
    let total_size_bytes: i64 =
        conn.query_row("SELECT COALESCE(SUM(size_bytes), 0) FROM venvs", [], |r| r.get(0))?;
    let last_scan: Option<String> =
        conn.query_row("SELECT MAX(scanned_at) FROM venvs", [], |r| r.get(0))?;

    Ok(ScanStatus {
        has_data: venv_count > 0,
        venv_count,
        package_count,
        total_size_bytes,
        last_scan,
    })
}

/// Get a flat export table with one row per venv/package/dependency combination.
pub fn get_export_rows(conn: &Connection) -> Result<Vec<ExportRow>> {
    // LEFT JOIN keeps venvs/packages visible even when package metadata does not
    // include dependency rows. That makes the export suitable for merge/dedup
    // analysis without losing sparse environments.
    let mut stmt = conn.prepare(
        "SELECT
            v.id,
            v.path,
            v.project_path,
            v.python_version,
            v.python_executable,
            v.venv_name,
            v.last_modified,
            v.scanned_at,
            v.config_files,
            (SELECT COUNT(*) FROM packages pc WHERE pc.venv_id = v.id) AS package_count,
            v.size_bytes,
            p.id,
            p.name,
            p.version,
            p.summary,
            d.id,
            d.dep_name,
            d.version_spec,
            d.extra,
            d.requires_raw
         FROM venvs v
         LEFT JOIN packages p ON p.venv_id = v.id
         LEFT JOIN dependencies d ON d.package_id = p.id
         ORDER BY v.project_path, v.venv_name, p.name, d.dep_name",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(ExportRow {
            venv_id: row.get(0)?,
            venv_path: row.get(1)?,
            project_path: row.get(2)?,
            python_version: row.get(3)?,
            python_executable: row.get(4)?,
            venv_name: row.get(5)?,
            last_modified: row.get(6)?,
            scanned_at: row.get(7)?,
            config_files: row.get(8)?,
            package_count: row.get(9)?,
            venv_size_bytes: row.get(10)?,
            package_id: row.get(11)?,
            package_name: row.get(12)?,
            package_version: row.get(13)?,
            package_summary: row.get(14)?,
            dependency_id: row.get(15)?,
            dependency_name: row.get(16)?,
            dependency_version_spec: row.get(17)?,
            dependency_extra: row.get(18)?,
            dependency_requires_raw: row.get(19)?,
        })
    })?;

    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service;

    #[test]
    fn export_rows_include_flat_venv_and_package_data() {
        let conn = service::open_demo_db().unwrap();
        let rows = get_export_rows(&conn).unwrap();

        assert!(!rows.is_empty());
        assert!(rows.iter().any(|row| {
            row.project_path == "/home/user/projects/web-dashboard"
                && row.package_name.as_deref() == Some("requests")
                && row.package_version.as_deref() == Some("2.32.3")
        }));
        assert!(rows.iter().all(|row| !row.venv_path.is_empty()));
        assert!(rows.iter().all(|row| row.package_count > 0));
        assert!(rows.iter().all(|row| row.venv_size_bytes > 0));
    }
}

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use rusqlite::{params, Connection, Error as SqliteError, ErrorCode};
use serde::Serialize;

use crate::db;
use crate::disk_usage;
use crate::export::{self, ExportFormat};
use crate::models::{Dependency, Package, PackageSearchResult, ScanStatus, Venv};
use crate::service::{self, ScanSummary};

#[derive(Debug)]
pub enum StartupMode {
    Gui { demo: bool },
    Cli(CliArgs),
}

#[derive(Debug, Parser)]
#[command(
    name = "sheepdog",
    version,
    about = "Python virtual environment manager",
    disable_help_subcommand = true
)]
struct RootArgs {
    #[arg(long, help = "Run the GUI with in-memory demo data")]
    demo: bool,

    #[command(subcommand)]
    command: Option<TopCommand>,
}

#[derive(Debug, Subcommand)]
enum TopCommand {
    #[command(about = "Run terminal commands without starting the GUI")]
    Cli(CliArgs),
}

#[derive(Debug, Args)]
pub struct CliArgs {
    #[arg(
        long,
        global = true,
        help = "Emit JSON instead of a human-readable table"
    )]
    json: bool,

    #[arg(long, global = true, help = "Use in-memory demo data")]
    demo: bool,

    #[command(subcommand)]
    command: CliCommand,
}

#[derive(Debug, Subcommand)]
enum CliCommand {
    #[command(about = "Show cache status")]
    Status,
    #[command(about = "Scan for virtual environments and rebuild the cache")]
    Scan {
        #[arg(help = "Root path to scan; defaults to your home directory")]
        root_path: Option<PathBuf>,
    },
    #[command(about = "List indexed virtual environments")]
    List,
    #[command(about = "List packages for a virtual environment")]
    Packages {
        #[arg(help = "Virtual environment id from `sheepdog cli list`")]
        venv_id: i64,
    },
    #[command(about = "Search packages across all virtual environments")]
    Search {
        #[arg(help = "Package name query")]
        query: String,
    },
    #[command(about = "List dependencies for a package")]
    Deps {
        #[arg(help = "Package id from `sheepdog cli packages` or `sheepdog cli search`")]
        package_id: i64,
    },
    #[command(about = "Export the consolidated venv/package/dependency table")]
    Export {
        #[arg(long, value_enum, default_value_t = ExportFormat::Csv, help = "Export format")]
        format: ExportFormat,
        #[arg(
            short,
            long,
            value_name = "PATH",
            help = "Output file path, or '-' for stdout; defaults to a timestamped file"
        )]
        output: Option<PathBuf>,
    },
}

#[derive(Debug)]
pub struct CliError {
    message: String,
    code: i32,
}

impl CliError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: 1,
        }
    }

    pub fn code(&self) -> i32 {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<SqliteError> for CliError {
    fn from(error: SqliteError) -> Self {
        Self::new(format_sqlite_error(error))
    }
}

impl From<String> for CliError {
    fn from(error: String) -> Self {
        Self::new(error)
    }
}

pub fn parse_startup_from<I, T>(args: I) -> Result<StartupMode, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let parsed = RootArgs::try_parse_from(args)?;
    match parsed.command {
        Some(TopCommand::Cli(cli_args)) => Ok(StartupMode::Cli(cli_args)),
        None => Ok(StartupMode::Gui { demo: parsed.demo }),
    }
}

pub fn run_cli(args: CliArgs) -> Result<(), CliError> {
    match args.command {
        CliCommand::Status => {
            let conn = open_db(args.demo)?;
            let status = db::get_scan_status(&conn)?;
            print_status(&status, args.json)
        }
        CliCommand::Scan { root_path } => {
            if args.demo {
                let conn = service::open_demo_db()?;
                let status = db::get_scan_status(&conn)?;
                let summary = ScanSummary {
                    root_path: "demo".to_string(),
                    discovered: status.venv_count as usize,
                    indexed: status.venv_count as usize,
                };
                return print_scan_summary(&summary, args.json);
            }

            let conn = service::open_cache_db()?;
            let root = root_path.unwrap_or_else(|| {
                dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/"))
            });
            let summary = service::scan_and_index(&conn, &root, |_| {})?;
            print_scan_summary(&summary, args.json)
        }
        CliCommand::List => {
            let conn = open_db(args.demo)?;
            let venvs = db::get_all_venvs(&conn)?;
            print_venvs(&venvs, args.json)
        }
        CliCommand::Packages { venv_id } => {
            let conn = open_db(args.demo)?;
            ensure_venv_exists(&conn, venv_id)?;
            let packages = db::get_venv_packages(&conn, venv_id)?;
            print_packages(&packages, args.json)
        }
        CliCommand::Search { query } => {
            let conn = open_db(args.demo)?;
            let results = db::search_packages(&conn, &query)?;
            print_search_results(&results, args.json)
        }
        CliCommand::Deps { package_id } => {
            let conn = open_db(args.demo)?;
            ensure_package_exists(&conn, package_id)?;
            let deps = db::get_package_dependencies(&conn, package_id)?;
            print_dependencies(&deps, args.json)
        }
        CliCommand::Export { format, output } => {
            let conn = open_db(args.demo)?;
            let rows = db::get_export_rows(&conn)?;
            let written = export::write_export(&rows, format, output)?;

            if let Some(path) = written {
                println!("Exported {} rows to {}", rows.len(), path.display());
            }

            Ok(())
        }
    }
}

fn open_db(demo: bool) -> Result<Connection, CliError> {
    if demo {
        service::open_demo_db().map_err(CliError::from)
    } else {
        service::open_cache_db().map_err(CliError::from)
    }
}

fn ensure_venv_exists(conn: &Connection, venv_id: i64) -> Result<(), CliError> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM venvs WHERE id = ?1)",
        params![venv_id],
        |row| row.get(0),
    )?;

    if exists {
        Ok(())
    } else {
        Err(CliError::new(format!(
            "No virtual environment found with id {venv_id}"
        )))
    }
}

fn ensure_package_exists(conn: &Connection, package_id: i64) -> Result<(), CliError> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM packages WHERE id = ?1)",
        params![package_id],
        |row| row.get(0),
    )?;

    if exists {
        Ok(())
    } else {
        Err(CliError::new(format!(
            "No package found with id {package_id}"
        )))
    }
}

fn print_status(status: &ScanStatus, json: bool) -> Result<(), CliError> {
    if json {
        return print_json(status);
    }

    print_table(
        &["has_data", "venvs", "packages", "disk_size", "last_scan"],
        vec![vec![
            status.has_data.to_string(),
            status.venv_count.to_string(),
            status.package_count.to_string(),
            disk_usage::format_bytes(status.total_size_bytes as u64),
            status.last_scan.clone().unwrap_or_else(|| "-".to_string()),
        ]],
    );
    Ok(())
}

fn print_scan_summary(summary: &ScanSummary, json: bool) -> Result<(), CliError> {
    if json {
        return print_json(summary);
    }

    print_table(
        &["root", "discovered", "indexed"],
        vec![vec![
            summary.root_path.clone(),
            summary.discovered.to_string(),
            summary.indexed.to_string(),
        ]],
    );
    Ok(())
}

fn print_venvs(venvs: &[Venv], json: bool) -> Result<(), CliError> {
    if json {
        return print_json(venvs);
    }

    let rows = venvs
        .iter()
        .map(|venv| {
            vec![
                venv.id.to_string(),
                venv.project_path.clone(),
                venv.venv_name.clone(),
                venv.python_version.clone(),
                venv.package_count.to_string(),
                disk_usage::format_bytes(venv.size_bytes as u64),
                venv.scanned_at.clone(),
            ]
        })
        .collect();
    print_table(
        &["id", "project", "venv", "python", "packages", "disk_size", "scanned_at"],
        rows,
    );
    Ok(())
}

fn print_packages(packages: &[Package], json: bool) -> Result<(), CliError> {
    if json {
        return print_json(packages);
    }

    let rows = packages
        .iter()
        .map(|package| {
            vec![
                package.id.to_string(),
                package.name.clone(),
                package.version.clone(),
                package.summary.clone().unwrap_or_default(),
            ]
        })
        .collect();
    print_table(&["id", "name", "version", "summary"], rows);
    Ok(())
}

fn print_search_results(results: &[PackageSearchResult], json: bool) -> Result<(), CliError> {
    if json {
        return print_json(results);
    }

    let rows = results
        .iter()
        .map(|result| {
            vec![
                result.package_id.to_string(),
                result.venv_id.to_string(),
                result.package_name.clone(),
                result.package_version.clone(),
                result.venv_name.clone(),
                result.project_path.clone(),
            ]
        })
        .collect();
    print_table(
        &["pkg_id", "venv_id", "name", "version", "venv", "project"],
        rows,
    );
    Ok(())
}

fn print_dependencies(deps: &[Dependency], json: bool) -> Result<(), CliError> {
    if json {
        return print_json(deps);
    }

    let rows = deps
        .iter()
        .map(|dep| {
            vec![
                dep.id.to_string(),
                dep.dep_name.clone(),
                dep.version_spec.clone().unwrap_or_default(),
                dep.extra.clone().unwrap_or_default(),
                dep.requires_raw.clone(),
            ]
        })
        .collect();
    print_table(&["id", "name", "version_spec", "extra", "raw"], rows);
    Ok(())
}

fn print_json<T: Serialize + ?Sized>(value: &T) -> Result<(), CliError> {
    let body = serde_json::to_string_pretty(value)
        .map_err(|e| CliError::new(format!("Failed to serialize JSON: {e}")))?;
    println!("{body}");
    Ok(())
}

fn print_table(headers: &[&str], rows: Vec<Vec<String>>) {
    if rows.is_empty() {
        println!("No results.");
        return;
    }

    let mut widths: Vec<usize> = headers.iter().map(|header| header.len()).collect();
    for row in &rows {
        for (idx, cell) in row.iter().enumerate() {
            widths[idx] = widths[idx].max(cell.len());
        }
    }

    print_row(headers.iter().map(|s| s.to_string()).collect(), &widths);
    print_separator(&widths);
    for row in rows {
        print_row(row, &widths);
    }
}

fn print_row(row: Vec<String>, widths: &[usize]) {
    for (idx, cell) in row.iter().enumerate() {
        if idx > 0 {
            print!("  ");
        }
        print!("{cell:<width$}", width = widths[idx]);
    }
    println!();
}

fn print_separator(widths: &[usize]) {
    for (idx, width) in widths.iter().enumerate() {
        if idx > 0 {
            print!("  ");
        }
        print!("{}", "-".repeat(*width));
    }
    println!();
}

fn format_sqlite_error(error: SqliteError) -> String {
    match error {
        SqliteError::SqliteFailure(ref sqlite_error, _)
            if sqlite_error.code == ErrorCode::DatabaseBusy =>
        {
            "Sheepdog database is locked; close Sheepdog or wait for the current scan to finish"
                .to_string()
        }
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_startup_from, StartupMode};

    #[test]
    fn no_args_launches_gui() {
        let mode = parse_startup_from(["sheepdog"]).unwrap();
        assert!(matches!(mode, StartupMode::Gui { demo: false }));
    }

    #[test]
    fn demo_arg_launches_gui_demo() {
        let mode = parse_startup_from(["sheepdog", "--demo"]).unwrap();
        assert!(matches!(mode, StartupMode::Gui { demo: true }));
    }

    #[test]
    fn cli_status_routes_to_cli() {
        let mode = parse_startup_from(["sheepdog", "cli", "status"]).unwrap();
        assert!(matches!(mode, StartupMode::Cli(_)));
    }

    #[test]
    fn cli_global_flags_parse_before_command() {
        let mode = parse_startup_from(["sheepdog", "cli", "--demo", "--json", "list"]).unwrap();
        assert!(matches!(mode, StartupMode::Cli(_)));
    }

    #[test]
    fn cli_global_flags_parse_after_command() {
        let mode = parse_startup_from(["sheepdog", "cli", "search", "requests", "--json"]).unwrap();
        assert!(matches!(mode, StartupMode::Cli(_)));
    }

    #[test]
    fn cli_export_uses_default_csv_file_behavior() {
        let mode = parse_startup_from(["sheepdog", "cli", "export"]).unwrap();
        match mode {
            StartupMode::Cli(args) => assert!(matches!(args.command, super::CliCommand::Export { .. })),
            _ => panic!("expected cli mode"),
        }
    }

    #[test]
    fn cli_export_accepts_json_output_path() {
        let mode = parse_startup_from([
            "sheepdog",
            "cli",
            "export",
            "--format",
            "json",
            "--output",
            "out.json",
        ])
        .unwrap();
        assert!(matches!(mode, StartupMode::Cli(_)));
    }

    #[test]
    fn cli_export_accepts_demo_stdout_csv() {
        let mode = parse_startup_from([
            "sheepdog", "cli", "--demo", "export", "--format", "csv", "--output", "-",
        ])
        .unwrap();
        assert!(matches!(mode, StartupMode::Cli(_)));
    }
}

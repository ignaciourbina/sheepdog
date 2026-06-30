use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::ValueEnum;

use crate::models::ExportRow;

const EXPORT_FILENAME_PREFIX: &str = "sheepdog-consolidated";

// Keep this order synchronized with src/export-config.js. The GUI and CLI use
// separate runtimes, so this is the stable contract for CSV consumers.
pub const EXPORT_COLUMNS: &[&str] = &[
    "venv_id",
    "venv_path",
    "project_path",
    "python_version",
    "python_executable",
    "venv_name",
    "last_modified",
    "scanned_at",
    "config_files",
    "package_count",
    "venv_size_bytes",
    "package_id",
    "package_name",
    "package_version",
    "package_summary",
    "dependency_id",
    "dependency_name",
    "dependency_version_spec",
    "dependency_extra",
    "dependency_requires_raw",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ExportFormat {
    Csv,
    Json,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Json => "json",
        }
    }
}

pub fn default_export_path(format: ExportFormat) -> PathBuf {
    let timestamp = chrono::Local::now().format("%Y-%m-%dT%H-%M-%S");
    PathBuf::from(format!(
        "{EXPORT_FILENAME_PREFIX}-{timestamp}.{}",
        format.extension()
    ))
}

pub fn write_export(
    rows: &[ExportRow],
    format: ExportFormat,
    output: Option<PathBuf>,
) -> Result<Option<PathBuf>, String> {
    let content = serialize_export(rows, format)?;

    match output {
        Some(path) if path.as_os_str() == "-" => {
            let mut stdout = io::stdout();
            stdout
                .write_all(content.as_bytes())
                .map_err(|e| format!("Failed to write export to stdout: {e}"))?;
            Ok(None)
        }
        Some(path) => {
            write_file(&path, &content)?;
            Ok(Some(path))
        }
        None => {
            let path = default_export_path(format);
            write_file(&path, &content)?;
            Ok(Some(path))
        }
    }
}

pub fn serialize_export(rows: &[ExportRow], format: ExportFormat) -> Result<String, String> {
    match format {
        ExportFormat::Csv => Ok(rows_to_csv(rows)),
        ExportFormat::Json => serde_json::to_string_pretty(rows)
            .map(|json| format!("{json}\n"))
            .map_err(|e| format!("Failed to serialize export JSON: {e}")),
    }
}

fn write_file(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(format!(
                "Output directory does not exist: {}",
                parent.display()
            ));
        }
    }

    std::fs::write(path, content)
        .map_err(|e| format!("Failed to write export to {}: {e}", path.display()))
}

fn rows_to_csv(rows: &[ExportRow]) -> String {
    let mut lines = Vec::with_capacity(rows.len() + 1);
    lines.push(EXPORT_COLUMNS.join(","));

    for row in rows {
        lines.push(
            export_values(row)
                .into_iter()
                .map(csv_cell)
                .collect::<Vec<_>>()
                .join(","),
        );
    }

    lines.join("\n") + "\n"
}

fn export_values(row: &ExportRow) -> Vec<String> {
    vec![
        row.venv_id.to_string(),
        row.venv_path.clone(),
        row.project_path.clone(),
        row.python_version.clone(),
        row.python_executable.clone(),
        row.venv_name.clone(),
        row.last_modified.clone(),
        row.scanned_at.clone(),
        row.config_files.clone(),
        row.package_count.to_string(),
        row.venv_size_bytes.to_string(),
        optional_to_string(row.package_id),
        row.package_name.clone().unwrap_or_default(),
        row.package_version.clone().unwrap_or_default(),
        row.package_summary.clone().unwrap_or_default(),
        optional_to_string(row.dependency_id),
        row.dependency_name.clone().unwrap_or_default(),
        row.dependency_version_spec.clone().unwrap_or_default(),
        row.dependency_extra.clone().unwrap_or_default(),
        row.dependency_requires_raw.clone().unwrap_or_default(),
    ]
}

fn optional_to_string<T: ToString>(value: Option<T>) -> String {
    value.map(|v| v.to_string()).unwrap_or_default()
}

fn csv_cell(value: String) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn export_row_with_values(package_summary: Option<String>) -> ExportRow {
        ExportRow {
            venv_id: 1,
            venv_path: "/tmp/project/.venv".to_string(),
            project_path: "/tmp/project".to_string(),
            python_version: "3.13.5".to_string(),
            python_executable: "/tmp/project/.venv/bin/python".to_string(),
            venv_name: ".venv".to_string(),
            last_modified: "2026-03-15 10:30".to_string(),
            scanned_at: "2026-03-27 19:30:00".to_string(),
            config_files: "requirements.txt,pyproject.toml".to_string(),
            package_count: 1,
            venv_size_bytes: 42 * 1024 * 1024,
            package_id: Some(2),
            package_name: Some("requests".to_string()),
            package_version: Some("2.32.3".to_string()),
            package_summary,
            dependency_id: None,
            dependency_name: None,
            dependency_version_spec: None,
            dependency_extra: None,
            dependency_requires_raw: None,
        }
    }

    #[test]
    fn csv_export_includes_stable_header() {
        let csv = serialize_export(&[export_row_with_values(None)], ExportFormat::Csv).unwrap();
        assert!(csv.starts_with(&format!("{}\n", EXPORT_COLUMNS.join(","))));
        assert!(csv.contains("requests"));
    }

    #[test]
    fn csv_export_escapes_commas_quotes_and_newlines() {
        let csv = serialize_export(
            &[export_row_with_values(Some("Hello, \"world\"\nNext".to_string()))],
            ExportFormat::Csv,
        )
        .unwrap();

        assert!(csv.contains("\"requirements.txt,pyproject.toml\""));
        assert!(csv.contains("\"Hello, \"\"world\"\"\nNext\""));
    }

    #[test]
    fn json_export_is_valid_json_array() {
        let json = serialize_export(&[export_row_with_values(None)], ExportFormat::Json).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.as_array().is_some());
    }
}

// Stable column order for the consolidated export. Keep this in sync with
// the Rust ExportRow model so CSV headers and JSON object keys describe
// the same flat table.
export const EXPORT_COLUMNS = [
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

export const EXPORT_FILENAME_PREFIX = "sheepdog-consolidated";

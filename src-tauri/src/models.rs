use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Venv {
    pub id: i64,
    pub path: String,
    pub project_path: String,
    pub python_version: String,
    pub python_executable: String,
    pub venv_name: String,
    pub last_modified: String,
    pub scanned_at: String,
    pub package_count: i64,
    pub size_bytes: i64,
    pub config_files: String, // comma-separated: requirements.txt,pyproject.toml,...
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub id: i64,
    pub venv_id: i64,
    pub name: String,
    pub version: String,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub id: i64,
    pub package_id: i64,
    pub requires_raw: String,
    pub dep_name: String,
    pub version_spec: Option<String>,
    pub extra: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSearchResult {
    pub package_name: String,
    pub package_version: String,
    pub venv_path: String,
    pub project_path: String,
    pub python_version: String,
    pub venv_name: String,
    pub venv_id: i64,
    pub package_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanStatus {
    pub has_data: bool,
    pub venv_count: i64,
    pub package_count: i64,
    pub total_size_bytes: i64,
    pub last_scan: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub total_found: usize,
    pub current: usize,
    pub current_path: String,
    pub phase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PypiVersionInfo {
    pub package_name: String,
    pub installed_version: String,
    pub latest_version: Option<String>,
    pub is_outdated: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRow {
    // This is intentionally denormalized for downstream tools: each row has
    // enough venv, package, and dependency context to be analyzed independently.
    pub venv_id: i64,
    pub venv_path: String,
    pub project_path: String,
    pub python_version: String,
    pub python_executable: String,
    pub venv_name: String,
    pub last_modified: String,
    pub scanned_at: String,
    pub config_files: String,
    pub package_count: i64,
    pub venv_size_bytes: i64,
    pub package_id: Option<i64>,
    pub package_name: Option<String>,
    pub package_version: Option<String>,
    pub package_summary: Option<String>,
    pub dependency_id: Option<i64>,
    pub dependency_name: Option<String>,
    pub dependency_version_spec: Option<String>,
    pub dependency_extra: Option<String>,
    pub dependency_requires_raw: Option<String>,
}

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

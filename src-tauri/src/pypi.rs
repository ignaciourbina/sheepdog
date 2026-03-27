use std::collections::HashMap;
use std::sync::Mutex;

use crate::models::PypiVersionInfo;

pub struct PypiCache {
    pub cache: Mutex<HashMap<String, String>>,
}

impl PypiCache {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }
}

/// Fetch the latest version of a package from PyPI.
async fn fetch_latest_version(name: &str) -> Result<String, String> {
    let url = format!("https://pypi.org/pypi/{}/json", name);
    let resp = reqwest::get(&url)
        .await
        .map_err(|e| format!("Request failed for {}: {}", name, e))?;

    if !resp.status().is_success() {
        return Err(format!("PyPI returned {} for {}", resp.status(), name));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse JSON for {}: {}", name, e))?;

    json["info"]["version"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("No version found for {}", name))
}

/// Check a list of packages against PyPI and return version info.
pub async fn check_packages(
    packages: Vec<(String, String)>, // (name, installed_version)
    cache: &PypiCache,
) -> Vec<PypiVersionInfo> {
    let mut results = Vec::new();

    for (name, installed) in packages {
        // Check cache first
        {
            let c = cache.cache.lock().unwrap();
            if let Some(latest) = c.get(&name.to_lowercase()) {
                results.push(PypiVersionInfo {
                    package_name: name.clone(),
                    installed_version: installed.clone(),
                    latest_version: Some(latest.clone()),
                    is_outdated: *latest != installed,
                    error: None,
                });
                continue;
            }
        }

        match fetch_latest_version(&name).await {
            Ok(latest) => {
                let is_outdated = latest != installed;
                // Cache the result
                {
                    let mut c = cache.cache.lock().unwrap();
                    c.insert(name.to_lowercase(), latest.clone());
                }
                results.push(PypiVersionInfo {
                    package_name: name,
                    installed_version: installed,
                    latest_version: Some(latest),
                    is_outdated,
                    error: None,
                });
            }
            Err(e) => {
                results.push(PypiVersionInfo {
                    package_name: name,
                    installed_version: installed,
                    latest_version: None,
                    is_outdated: false,
                    error: Some(e),
                });
            }
        }
    }

    results
}

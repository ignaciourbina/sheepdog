use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Parsed contents of a pyvenv.cfg file.
#[derive(Debug)]
pub struct PyvenvConfig {
    pub version: String,
    pub executable: String,
    pub home: String,
    pub include_system_site_packages: bool,
}

/// Parsed package metadata from a .dist-info/METADATA file.
#[derive(Debug)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub summary: Option<String>,
    pub requires_dist: Vec<String>,
}

/// Parsed Requires-Dist entry.
#[derive(Debug)]
pub struct ParsedRequirement {
    pub name: String,
    pub version_spec: Option<String>,
    pub extra: Option<String>,
}

/// Parse a pyvenv.cfg file into its key-value pairs.
pub fn parse_pyvenv_cfg(venv_path: &Path) -> Option<PyvenvConfig> {
    let cfg_path = venv_path.join("pyvenv.cfg");
    let content = fs::read_to_string(&cfg_path).ok()?;

    let mut map = HashMap::new();
    for line in content.lines() {
        if let Some((key, value)) = line.split_once('=') {
            map.insert(key.trim().to_lowercase(), value.trim().to_string());
        }
    }

    Some(PyvenvConfig {
        version: map.get("version").cloned().unwrap_or_default(),
        executable: map
            .get("executable")
            .or_else(|| map.get("home"))
            .cloned()
            .unwrap_or_default(),
        home: map.get("home").cloned().unwrap_or_default(),
        include_system_site_packages: map
            .get("include-system-site-packages")
            .map(|v| v == "true")
            .unwrap_or(false),
    })
}

/// Find and parse all .dist-info/METADATA files in a venv's site-packages.
pub fn parse_packages(venv_path: &Path) -> Vec<PackageInfo> {
    let mut packages = Vec::new();

    // Determine the site-packages path: lib/pythonX.Y/site-packages/
    let lib_dir = venv_path.join("lib");
    let site_packages = if let Ok(entries) = fs::read_dir(&lib_dir) {
        entries
            .flatten()
            .find(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("python")
            })
            .map(|e| e.path().join("site-packages"))
    } else {
        None
    };

    let site_packages = match site_packages {
        Some(sp) if sp.is_dir() => sp,
        _ => return packages,
    };

    // Find all .dist-info directories
    let entries = match fs::read_dir(&site_packages) {
        Ok(entries) => entries,
        Err(_) => return packages,
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".dist-info") && entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            let metadata_path = entry.path().join("METADATA");
            if let Some(pkg) = parse_metadata(&metadata_path) {
                packages.push(pkg);
            }
        }
    }

    packages
}

/// Parse a single METADATA file.
fn parse_metadata(path: &Path) -> Option<PackageInfo> {
    let content = fs::read_to_string(path).ok()?;

    let mut name = None;
    let mut version = None;
    let mut summary = None;
    let mut requires_dist = Vec::new();

    for line in content.lines() {
        // METADATA headers end at the first blank line (body follows)
        if line.is_empty() {
            break;
        }

        if let Some(val) = line.strip_prefix("Name: ") {
            name = Some(val.trim().to_string());
        } else if let Some(val) = line.strip_prefix("Version: ") {
            version = Some(val.trim().to_string());
        } else if let Some(val) = line.strip_prefix("Summary: ") {
            summary = Some(val.trim().to_string());
        } else if let Some(val) = line.strip_prefix("Requires-Dist: ") {
            requires_dist.push(val.trim().to_string());
        }
    }

    Some(PackageInfo {
        name: name?,
        version: version.unwrap_or_else(|| "unknown".to_string()),
        summary,
        requires_dist,
    })
}

/// Parse a Requires-Dist string into structured parts.
/// Examples:
///   "anyio >=3.5.0,<5"
///   "aiohttp ; extra == \"aiohttp\""
///   "numpy (>=1.21) ; extra == \"datalib\""
pub fn parse_requires_dist(raw: &str) -> ParsedRequirement {
    let raw = raw.trim();

    // Split on ';' to separate the extra condition
    let (main_part, extra) = if let Some(idx) = raw.find(';') {
        let (main, cond) = raw.split_at(idx);
        let cond = cond[1..].trim();
        let extra = extract_extra(cond);
        (main.trim(), extra)
    } else {
        (raw, None)
    };

    // Split the main part into name and version spec
    // The version spec can start with ( or with a comparison operator
    let (name, version_spec) = if let Some(idx) = main_part.find('(') {
        let name = main_part[..idx].trim();
        let spec = main_part[idx..]
            .trim()
            .trim_start_matches('(')
            .trim_end_matches(')');
        (name.to_string(), Some(spec.trim().to_string()))
    } else if let Some(idx) = main_part.find(|c: char| c == '<' || c == '>' || c == '=' || c == '!' || c == '~') {
        let name = main_part[..idx].trim();
        let spec = main_part[idx..].trim();
        (name.to_string(), Some(spec.to_string()))
    } else {
        (main_part.trim().to_string(), None)
    };

    // Clean up version spec — remove empty ones
    let version_spec = version_spec.filter(|s| !s.is_empty());

    ParsedRequirement {
        name,
        version_spec,
        extra,
    }
}

/// Extract the extra name from a condition like `extra == "aiohttp"` or `extra == 'aiohttp'`.
fn extract_extra(condition: &str) -> Option<String> {
    if condition.contains("extra") {
        // Find quoted string
        let start = condition.find(|c| c == '"' || c == '\'');
        if let Some(start) = start {
            let quote = condition.as_bytes()[start] as char;
            let rest = &condition[start + 1..];
            if let Some(end) = rest.find(quote) {
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}

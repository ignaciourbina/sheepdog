use std::path::Path;

use walkdir::WalkDir;

/// Measure the allocated disk space below `root`, like `du`.
///
/// Symlinks are not followed: Sheepdog reports the space owned by a venv
/// directory without accidentally traversing shared caches or unrelated trees.
/// Unreadable entries are skipped so one protected file does not abort a scan.
pub fn directory_size_bytes(root: &Path) -> u64 {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.metadata().ok())
        .map(|metadata| allocated_bytes(&metadata))
        .sum()
}

#[cfg(unix)]
fn allocated_bytes(metadata: &std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;

    metadata.blocks().saturating_mul(512)
}

#[cfg(not(unix))]
fn allocated_bytes(metadata: &std::fs::Metadata) -> u64 {
    metadata.len()
}

/// Format bytes compactly for terminal tables while preserving raw bytes in
/// JSON and CSV output for downstream analysis.
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;

    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_human_readable_sizes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1024 * 1024 * 3), "3.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024 * 2), "2.0 GB");
    }
}

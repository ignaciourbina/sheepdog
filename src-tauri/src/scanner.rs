use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Find all Python virtual environments under `root` by looking for `pyvenv.cfg` files.
pub fn find_venvs(root: &Path) -> Vec<PathBuf> {
    let mut venvs = Vec::new();

    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            // Skip directories that are never venvs and would slow the scan
            if entry.file_type().is_dir() {
                // Skip known irrelevant directories
                if matches!(
                    name.as_ref(),
                    "node_modules"
                        | "__pycache__"
                        | ".git"
                        | ".hg"
                        | ".svn"
                        | "target"
                        | ".tox"
                        | ".mypy_cache"
                        | ".pytest_cache"
                        | ".ruff_cache"
                        | "dist"
                        | "build"
                        | ".cargo"
                        | ".cache"
                        | "Trash"
                        | "snap"
                ) {
                    return false;
                }

                // Skip .local/share/Trash (system trash)
                let path_str = entry.path().to_string_lossy();
                if path_str.contains("/Trash/") || path_str.contains("/.Trash") {
                    return false;
                }

                return true;
            }
            true
        });

    for entry in walker.flatten() {
        if entry.file_type().is_file() && entry.file_name() == "pyvenv.cfg" {
            if let Some(venv_dir) = entry.path().parent() {
                venvs.push(venv_dir.to_path_buf());
            }
        }
    }

    venvs
}

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
                return !matches!(
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
                );
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

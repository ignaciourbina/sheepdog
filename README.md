# Sheepdog

A lightweight desktop app that scans your machine for Python virtual environments, indexes their packages into a local database, and gives you a fast, searchable GUI to browse and manage them.

Built with **Rust** (backend) and **Tauri v2** (native webview UI). Single binary, ~16MB, no Electron.

![Sheepdog screenshot](https://raw.githubusercontent.com/ignaciourbina/sheepdog/main/docs/screenshot.png)

## What it does

- **Scan** your filesystem for all Python venvs (finds `pyvenv.cfg` files)
- **Index** every installed package, version, and dependency into SQLite
- **Search** across all venvs instantly — "which repos have `otree` installed?"
- **Compare** two venvs side by side — see shared packages, version differences, and exclusives
- **Check outdated** packages against PyPI
- **Inspect** config files (`requirements.txt`, `pyproject.toml`, etc.) inline
- **Open in VS Code** directly from the context menu

## Install

### Prerequisites

- **Rust** (1.70+) — [rustup.rs](https://rustup.rs)
- **Node.js** (18+) and npm
- **System libraries** (Ubuntu/Debian):

```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev
```

### Build and install

```bash
git clone https://github.com/ignaciourbina/sheepdog.git
cd sheepdog
./install.sh
```

This builds a release binary and installs it to `~/.local/bin/sheepdog`. It also adds a `.desktop` file so Sheepdog appears in your app launcher.

### Run

```bash
sheepdog
```

Or find **Sheepdog** in your desktop application launcher.

### Uninstall

```bash
./uninstall.sh
```

## Usage

1. Click **Scan** to discover all venvs under your home directory
2. **Click a row** to expand and see its packages inline
3. **Search** for a package name — see every venv that has it
4. **Right-click** a row for actions:
   - Open in VS Code
   - Copy project path
   - Select for side-by-side comparison
5. Click green **config badges** (req, pyproject, etc.) to view the file contents
6. Use **Python version filter chips** to narrow down the list
7. **Check Outdated** to compare installed versions against PyPI

### Keyboard shortcuts

| Key | Action |
|-----|--------|
| `/` | Focus search |
| `Escape` | Clear search / close modal |
| `j` / `k` or arrows | Navigate rows |
| `Enter` | Expand/collapse row |

## How it works

**Scanner** — Walks your filesystem with `walkdir`, looking for `pyvenv.cfg` files. Skips `node_modules`, `.git`, `__pycache__`, Trash, and snap directories.

**Parser** — Reads `pyvenv.cfg` for Python version, then walks `lib/pythonX.Y/site-packages/*.dist-info/METADATA` to extract package names, versions, summaries, and dependencies (`Requires-Dist`).

**Database** — Everything is cached in SQLite at `~/.cache/sheepdog/sheepdog.db`. The app loads instantly from cache; re-scan only when you click Scan.

**Frontend** — Vanilla HTML/CSS/JS (no framework). Communicates with the Rust backend via Tauri's IPC (`invoke()`).

## Project structure

```
sheepdog/
├── src/                    # Frontend (HTML/CSS/JS)
│   ├── index.html
│   ├── styles.css
│   └── main.js
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── lib.rs          # Tauri app setup
│   │   ├── commands.rs     # IPC command handlers
│   │   ├── scanner.rs      # Filesystem walker
│   │   ├── parser.rs       # pyvenv.cfg + METADATA parser
│   │   ├── db.rs           # SQLite schema + queries
│   │   ├── pypi.rs         # PyPI API client
│   │   └── models.rs       # Data structures
│   ├── Cargo.toml
│   └── tauri.conf.json
├── install.sh
├── uninstall.sh
└── package.json
```

## License

MIT

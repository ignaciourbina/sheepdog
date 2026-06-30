# Sheepdog

A lightweight desktop app that scans your machine for Python virtual environments, indexes their packages into a local database, and gives you a fast, searchable GUI to browse and manage them.

Built with **Rust** (backend) and **Tauri v2** (native webview UI). Single binary, ~16MB, no Electron.

![Sheepdog screenshot](https://raw.githubusercontent.com/ignaciourbina/sheepdog/main/docs/screenshot.png)

## What it does

- **Scan** your filesystem for all Python venvs (finds `pyvenv.cfg` files)
- **Index** every installed package, version, and dependency into SQLite
- **Measure** allocated disk space for each venv so large environments are easy to identify
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

### CLI

Sheepdog also includes a terminal interface under the `cli` subcommand. The GUI still opens when you run `sheepdog` with no arguments.

```bash
sheepdog cli status
sheepdog cli scan ~/projects
sheepdog cli list
sheepdog cli packages 1
sheepdog cli search requests
sheepdog cli deps 42
sheepdog cli export
```

Use `--json` with table-style CLI commands for scripting:

```bash
sheepdog cli search django --json
sheepdog cli list --json
```

Export the consolidated venv/package/dependency table for downstream analysis:

```bash
sheepdog cli export --format csv
sheepdog cli export --format json --output sheepdog-export.json
sheepdog cli export --format csv --output -
```

Use demo data without touching the cache database:

```bash
sheepdog cli --demo status
sheepdog cli --demo list
sheepdog cli --demo export --format json --output demo-export.json
```

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

**Scanner** — Walks your filesystem with `walkdir`, looking for `pyvenv.cfg` files. Skips `node_modules`, `.git`, `__pycache__`, Trash, and snap directories. During indexing, it records du-style allocated bytes for each venv.

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

## Built with

Vibecoded with [Claude Code](https://claude.ai/claude-code) (Claude Opus 4.6). Architecture, Rust backend, frontend UI, pixel art icon, and install scripts were pair-programmed in a single session.

## License

MIT

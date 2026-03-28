#!/usr/bin/env bash
set -euo pipefail

APP_NAME="sheepdog"
BIN_DIR="$HOME/.local/bin"
DATA_DIR="$HOME/.local/share/$APP_NAME"
CACHE_DIR="$HOME/.cache/$APP_NAME"
DESKTOP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/128x128/apps"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[+]${NC} $1"; }
warn()  { echo -e "${YELLOW}[!]${NC} $1"; }
error() { echo -e "${RED}[x]${NC} $1"; exit 1; }

# ── Check prerequisites ────────────────────────────────────────────

info "Checking prerequisites..."

command -v cargo >/dev/null 2>&1 || error "Rust/Cargo not found. Install from https://rustup.rs"
command -v npm >/dev/null 2>&1   || error "Node/npm not found. Install from https://nodejs.org"

# Check system libraries
missing_libs=()
for lib in webkit2gtk-4.1 gtk+-3.0 libsoup-3.0; do
  if ! pkg-config --exists "$lib" 2>/dev/null; then
    missing_libs+=("$lib")
  fi
done

if [ ${#missing_libs[@]} -gt 0 ]; then
  warn "Missing system libraries: ${missing_libs[*]}"
  echo ""
  echo "  On Ubuntu/Debian, install with:"
  echo "    sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev"
  echo ""
  read -p "  Continue anyway? [y/N] " -n 1 -r
  echo
  [[ $REPLY =~ ^[Yy]$ ]] || exit 1
fi

# ── Install npm dependencies ───────────────────────────────────────

info "Installing npm dependencies..."
npm install --silent

# ── Build release binary ───────────────────────────────────────────

info "Building release binary (this may take a few minutes on first build)..."

# Set PKG_CONFIG_PATH for systems where it's not automatically found
export PKG_CONFIG_PATH="${PKG_CONFIG_PATH:-}:/usr/lib/x86_64-linux-gnu/pkgconfig:/usr/share/pkgconfig"

npm run tauri build 2>&1 | tail -5

# Find the built binary
BUILT_BINARY="$(find "$CACHE_DIR/target" -path "*/release/$APP_NAME" -not -path "*/bundle/*" 2>/dev/null || true)"
if [ -z "$BUILT_BINARY" ]; then
  # Fallback: check default target dir
  BUILT_BINARY="src-tauri/target/release/$APP_NAME"
fi

[ -f "$BUILT_BINARY" ] || error "Build failed — binary not found at $BUILT_BINARY"

# ── Install ────────────────────────────────────────────────────────

info "Installing to $DATA_DIR..."

mkdir -p "$BIN_DIR" "$DATA_DIR" "$CACHE_DIR" "$DESKTOP_DIR" "$ICON_DIR"

# Copy binary
cp "$BUILT_BINARY" "$DATA_DIR/$APP_NAME"
chmod +x "$DATA_DIR/$APP_NAME"

# Copy icon
if [ -f "src-tauri/icons/128x128.png" ]; then
  cp "src-tauri/icons/128x128.png" "$ICON_DIR/$APP_NAME.png"
fi

# Create wrapper script (handles snap GTK env conflicts)
cat > "$BIN_DIR/$APP_NAME" << 'WRAPPER'
#!/usr/bin/env bash
# Sheepdog launcher — cleans GTK env vars that conflict with snap-installed apps
unset GTK_PATH GTK_EXE_PREFIX GIO_MODULE_DIR GTK_IM_MODULE_FILE GSETTINGS_SCHEMA_DIR
exec "$HOME/.local/share/sheepdog/sheepdog" "$@"
WRAPPER
chmod +x "$BIN_DIR/$APP_NAME"

# Create .desktop file
cat > "$DESKTOP_DIR/$APP_NAME.desktop" << DESKTOP
[Desktop Entry]
Name=Sheepdog
Comment=Python virtual environment manager
Exec=$BIN_DIR/$APP_NAME
Icon=$APP_NAME
Terminal=false
Type=Application
Categories=Development;Utility;
Keywords=python;venv;virtualenv;packages;
StartupWMClass=Sheepdog
DESKTOP

# Update desktop database
update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true

# ── Verify ─────────────────────────────────────────────────────────

echo ""
info "Sheepdog installed successfully!"
echo ""
echo "  Binary:   $DATA_DIR/$APP_NAME"
echo "  Launcher: $BIN_DIR/$APP_NAME"
echo "  Desktop:  $DESKTOP_DIR/$APP_NAME.desktop"
echo "  Database: $CACHE_DIR/$APP_NAME.db (created on first run)"
echo ""

if echo "$PATH" | tr ':' '\n' | grep -q "$BIN_DIR"; then
  info "Run 'sheepdog' from anywhere, or find it in your app launcher."
else
  warn "$BIN_DIR is not in your PATH."
  echo "  Add this to your ~/.bashrc or ~/.zshrc:"
  echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

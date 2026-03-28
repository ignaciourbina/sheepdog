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
NC='\033[0m'

echo "This will remove Sheepdog from your system."
echo ""
echo "  $BIN_DIR/$APP_NAME"
echo "  $DATA_DIR/"
echo "  $DESKTOP_DIR/$APP_NAME.desktop"
echo "  $ICON_DIR/$APP_NAME.png"
echo ""
read -p "Also remove cached data ($CACHE_DIR)? [y/N] " -n 1 -r
echo
REMOVE_CACHE=$REPLY

read -p "Proceed with uninstall? [y/N] " -n 1 -r
echo
[[ $REPLY =~ ^[Yy]$ ]] || exit 0

rm -f "$BIN_DIR/$APP_NAME"
rm -rf "$DATA_DIR"
rm -f "$DESKTOP_DIR/$APP_NAME.desktop"
rm -f "$ICON_DIR/$APP_NAME.png"

if [[ $REMOVE_CACHE =~ ^[Yy]$ ]]; then
  rm -rf "$CACHE_DIR"
fi

update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true

echo -e "${GREEN}[+]${NC} Sheepdog uninstalled."

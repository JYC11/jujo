#!/bin/sh
# Uninstall jujo
#
# Usage:
#   sh uninstall.sh              # removes from ~/.local/bin
#   sh uninstall.sh /usr/local/bin  # custom location

set -eu

INSTALL_DIR="${1:-$HOME/.local/bin}"
BINARY="jujo"
TARGET="$INSTALL_DIR/$BINARY"

if [ ! -f "$TARGET" ]; then
    echo "jujo not found at $TARGET — nothing to uninstall."
    exit 0
fi

rm "$TARGET"
echo "Removed $TARGET"
echo ""
echo "Note: existing .jujo/ project directories were NOT removed."
echo "To remove a project's data, delete its .jujo/ directory manually."

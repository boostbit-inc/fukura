#!/bin/sh
set -e

BIN_DIR="/usr/bin"

# Remove old versions from common locations
remove_old_versions() {
    echo "Checking for existing Fukura installations..."
    
    # Common installation locations
    for dir in "/usr/local/bin" "/usr/bin" "$HOME/.local/bin" "$HOME/.cargo/bin"; do
        for binary in "fuku" "fukura"; do
            if [ -f "$dir/$binary" ]; then
                # Check if it's not the current installation
                if [ "$dir" != "$BIN_DIR" ] || [ "$binary" != "fukura" ]; then
                    echo "Removing old version: $dir/$binary"
                    rm -f "$dir/$binary" 2>/dev/null || true
                fi
            fi
        done
    done
}

# Remove old versions before installing new one
remove_old_versions

# Create symlink for fuku command
if [ -x "$BIN_DIR/fukura" ]; then
  ln -sf "$BIN_DIR/fukura" "$BIN_DIR/fuku"
fi

# Update shell hash cache if present
if command -v hash >/dev/null 2>&1; then
  hash -r || true
fi

exit 0

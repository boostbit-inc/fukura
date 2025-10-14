#!/bin/sh
set -euo pipefail

PREFIX="/usr/local/bin"
INSTALL_ROOT="$TARGETROOT$PREFIX"

# Remove old versions from common locations
remove_old_versions() {
    echo "Checking for existing Fukura installations..."
    
    # Common installation locations
    for dir in "/usr/local/bin" "/usr/bin" "$HOME/.local/bin" "$HOME/.cargo/bin"; do
        for binary in "fuku" "fukura"; do
            if [ -f "$dir/$binary" ]; then
                # Check if it's not the current installation
                if [ "$dir" != "$INSTALL_ROOT" ] || [ "$binary" != "fukura" ]; then
                    echo "Removing old version: $dir/$binary"
                    rm -f "$dir/$binary" 2>/dev/null || true
                fi
            fi
        done
    done
}

# Remove old versions before installing new one
remove_old_versions

# Ensure installation directory exists
mkdir -p "$INSTALL_ROOT"

# Primary binary is installed as fukura automatically by cargo-dist/pkgbuild
# Ensure alias `fuku` exists as a symlink alongside it.
if [ -x "$INSTALL_ROOT/fukura" ]; then
  ln -sf "$PREFIX/fukura" "$INSTALL_ROOT/fuku"
fi

exit 0

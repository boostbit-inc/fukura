#!/bin/sh
set -euo pipefail

PREFIX="/usr/local/bin"
INSTALL_ROOT="$TARGETROOT$PREFIX"

# Ensure installation directory exists
mkdir -p "$INSTALL_ROOT"

# Primary binary is installed as fukura automatically by cargo-dist/pkgbuild
# Ensure alias `fuku` exists as a symlink alongside it.
if [ -x "$INSTALL_ROOT/fukura" ]; then
  ln -sf "$PREFIX/fukura" "$INSTALL_ROOT/fuku"
fi

exit 0

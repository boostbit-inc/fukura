#!/bin/sh
set -e

BIN_DIR="/usr/bin"
if [ -x "$BIN_DIR/fukura" ]; then
  ln -sf "$BIN_DIR/fukura" "$BIN_DIR/fuku"
fi

# Update shell hash cache if present
if command -v hash >/dev/null 2>&1; then
  hash -r || true
fi

exit 0

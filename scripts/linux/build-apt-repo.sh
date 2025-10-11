#!/usr/bin/env bash
set -euo pipefail

if ! command -v dpkg-scanpackages >/dev/null 2>&1; then
  echo "dpkg-scanpackages is required (apt install dpkg-dev)" >&2
  exit 1
fi

DIST_DIR=${1:-dist}
REPO_ROOT=${DIST_DIR}/apt
POOL_DIR=${REPO_ROOT}/pool/main/f/fukura
DIST_NAME=${2:-stable}
ARCH=${3:-amd64}

rm -rf "$REPO_ROOT"
mkdir -p "$POOL_DIR"

shopt -s nullglob
for pkg in ${DIST_DIR}/fukura_*_${ARCH}.deb; do
  cp "$pkg" "$POOL_DIR/"
  # copy corresponding signature if present
  if [[ -f "${pkg}.asc" ]]; then
    cp "${pkg}.asc" "$POOL_DIR/"
  fi
  echo "added $(basename "$pkg")"
done

DIST_DIR_DEB=pool/main/f/fukura
OUTPUT_DIR=${REPO_ROOT}/dists/${DIST_NAME}/main/binary-${ARCH}
mkdir -p "$OUTPUT_DIR"

if [ -f "${DIST_DIR}/fukura-archive-keyring.gpg" ]; then
  cp "${DIST_DIR}/fukura-archive-keyring.gpg" "$REPO_ROOT/fukura-archive-keyring.gpg"
fi

pushd "$REPO_ROOT" >/dev/null
PACKAGE_FILE=$(mktemp)
dpkg-scanpackages --arch "$ARCH" "$DIST_DIR_DEB" > "$PACKAGE_FILE"
gzip -9c "$PACKAGE_FILE" > "$OUTPUT_DIR/Packages.gz"
cp "$PACKAGE_FILE" "$OUTPUT_DIR/Packages"
rm "$PACKAGE_FILE"

cat > "$REPO_ROOT/dists/${DIST_NAME}/Release" <<REL
Origin: Fukura
Label: Fukura
Suite: ${DIST_NAME}
Version: 1.0
Codename: ${DIST_NAME}
Architectures: ${ARCH}
Components: main
Description: Fukura CLI APT repository
REL

popd >/dev/null

echo "APT repository staged at ${REPO_ROOT}"

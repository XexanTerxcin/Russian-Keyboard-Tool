#!/usr/bin/env bash
# Build the release binaries and a Debian package for Russian Keyboard Tool.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
export PATH="${HOME}/.cargo/bin:${PATH}"

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
ARCH="$(dpkg --print-architecture)"
STAGE="dist/deb-root"
DEB="dist/russian-keyboard-tool_${VERSION}_${ARCH}.deb"

echo "==> Building release binaries (v${VERSION}, ${ARCH})"
cargo build --release

echo "==> Staging package tree"
rm -rf "$STAGE"
mkdir -p "$STAGE"
cp -a packaging/deb/. "$STAGE/"

install -Dm755 target/release/russian-keyboard-tool "$STAGE/usr/bin/russian-keyboard-tool"
install -Dm755 target/release/russian-keyboard-engine "$STAGE/usr/bin/russian-keyboard-engine"
install -Dm644 resources/icons/icon-8.png "$STAGE/usr/share/pixmaps/russian-keyboard-tool.png"
install -Dm644 resources/icons/icon-8.png \
    "$STAGE/usr/share/icons/hicolor/256x256/apps/russian-keyboard-tool.png"
find "$STAGE" -type d -exec chmod 755 {} +
find "$STAGE" -type f -exec chmod 644 {} +
chmod 755 "$STAGE/DEBIAN/postinst" "$STAGE/DEBIAN/postrm" \
          "$STAGE/usr/bin/russian-keyboard-tool" \
          "$STAGE/usr/bin/russian-keyboard-engine" \
          "$STAGE/usr/bin/russian-keyboard-setup"
sed -i "s/@ARCH@/${ARCH}/" "$STAGE/DEBIAN/control"

echo "==> Building $DEB"
mkdir -p dist
dpkg-deb --root-owner-group --build "$STAGE" "$DEB"

echo
echo "Package: $DEB  ($(du -h "$DEB" | cut -f1))"
echo "Install: sudo apt install ./$DEB"

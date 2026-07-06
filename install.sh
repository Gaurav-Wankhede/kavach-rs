#!/usr/bin/env bash
# Kavach installer: detect OS+arch, download the matching release binary, install to ~/.local/bin.
set -euo pipefail

REPO="Wankhede-Brothers/kavach-rs"
BASE="https://github.com/${REPO}/releases/latest/download"
DEST="${KAVACH_INSTALL_DIR:-$HOME/.local/bin}"

os=$(uname -s); arch=$(uname -m)
case "$os" in
  Darwin) plat=darwin ;;
  Linux)  plat=linux ;;
  *) echo "kavach: unsupported OS '$os' — use install.ps1 on Windows" >&2; exit 1 ;;
esac
case "$arch" in
  x86_64|amd64)  cpu=amd64 ;;
  arm64|aarch64) cpu=arm64 ;;
  *) echo "kavach: unsupported arch '$arch'" >&2; exit 1 ;;
esac

asset="kavach-${plat}-${cpu}.tar.gz"
url="${BASE}/${asset}"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

echo "kavach: downloading ${asset} …"
if command -v curl >/dev/null 2>&1; then curl -fsSL -o "$tmp/k.tgz" "$url";
elif command -v wget >/dev/null 2>&1; then wget -qO "$tmp/k.tgz" "$url";
else echo "kavach: need curl or wget" >&2; exit 1; fi

tar -xzf "$tmp/k.tgz" -C "$tmp"
mkdir -p "$DEST"
install -m 0755 "$tmp/kavach" "$DEST/kavach"

echo "kavach: installed to ${DEST}/kavach"
case ":$PATH:" in *":$DEST:"*) ;; *) echo "kavach: add ${DEST} to PATH → export PATH=\"${DEST}:\$PATH\"" ;; esac
"$DEST/kavach" --version || true

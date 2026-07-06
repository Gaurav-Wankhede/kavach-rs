# Kavach source installer: clone, build from source, install to ~/.local/bin, delete the clone.
set -euo pipefail

REPO_URL="https://github.com/Wankhede-Brothers/kavach-rs"
DEST="${KAVACH_INSTALL_DIR:-$HOME/.local/bin}"

if ! command -v git >/dev/null 2>&1; then
  echo "kavach: git is required and was not found — install it first (Debian/Ubuntu: sudo apt install git; Fedora: sudo dnf install git; macOS: xcode-select --install)" >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "kavach: installing Rust via rustup ..."
  curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"
fi

if ! command -v surreal >/dev/null 2>&1; then
  echo "kavach: installing SurrealDB ..."
  curl -fsSL https://install.surrealdb.com | sh
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

echo "kavach: cloning ${REPO_URL} ..."
git clone --depth 1 "$REPO_URL" "$tmp/src"

echo "kavach: building kavach-cli (release) ..."
( cd "$tmp/src" && cargo build --release -p kavach-cli )

mkdir -p "$DEST"
install -m 0755 "$tmp/src/target/release/kavach" "$DEST/kavach"

echo "kavach: installed to ${DEST}/kavach"
case ":$PATH:" in *":$DEST:"*) ;; *) echo "kavach: add ${DEST} to PATH -> export PATH=\"${DEST}:\$PATH\"" ;; esac
"$DEST/kavach" --version || true
echo "kavach: update later with \`kavach update\` (no re-clone needed by you)."

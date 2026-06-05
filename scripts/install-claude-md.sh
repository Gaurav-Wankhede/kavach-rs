#!/usr/bin/env bash
# Install the global engineering directives to the user-global Claude config path.
# Path is derived from $HOME at runtime — nothing is hardcoded. Works on Linux and macOS.
set -euo pipefail

# Resolve the repo root from this script's own location (no absolute assumptions).
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
src="$script_dir/../assets/claude/CLAUDE.global.md"

if [ ! -f "$src" ]; then
  echo "error: source not found: $src" >&2
  exit 1
fi

home="${HOME:?HOME is not set}"
dest_dir="$home/.claude"
dest="$dest_dir/CLAUDE.md"

mkdir -p "$dest_dir"
cp "$src" "$dest"

echo "installed global CLAUDE.md -> $dest"

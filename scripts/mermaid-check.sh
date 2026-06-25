#!/usr/bin/env bash
set -euo pipefail
file="${1:?usage: mermaid-check.sh <file.html|file.md|file.mmd>}"
bun_bin="$(command -v bun || echo "$HOME/.bun/bin/bun")"
mmdc="$(command -v mmdc || echo "$HOME/.bun/bin/mmdc")"
[ -x "$bun_bin" ] || { echo "mermaid-check: bun absent" >&2; exit 127; }
[ -x "$mmdc" ] || { echo "mermaid-check: mmdc absent — bun add -g @mermaid-js/mermaid-cli" >&2; exit 127; }
work="$(mktemp -d)"; trap 'rm -rf "$work"' EXIT
md="$work/blocks.md"
n="$("$bun_bin" "$(dirname "$0")/mermaid-extract.js" "$file" "$md")"
[ "$n" -gt 0 ] || { echo "mermaid-check: no mermaid blocks in $file" >&2; exit 2; }
if "$mmdc" -i "$md" -o "$work/out.svg" >/dev/null 2>"$work/err"; then
  echo "mermaid-check: all $n block(s) valid in $file"
else
  echo "mermaid-check: SYNTAX ERROR in $file" >&2; sed 's/^/  /' "$work/err" >&2; exit 1
fi

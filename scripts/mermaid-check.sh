#!/usr/bin/env bash
set -euo pipefail
file="${1:?usage: mermaid-check.sh <file.html|file.mmd>}"
mmdc="$(command -v mmdc || echo "$HOME/.bun/bin/mmdc")"
[ -x "$mmdc" ] || { echo "mermaid-check: mmdc absent — bun add -g @mermaid-js/mermaid-cli" >&2; exit 127; }
work="$(mktemp -d)"; trap 'rm -rf "$work"' EXIT
rg -nU --multiline '(?s)<pre class="mermaid">(.*?)</pre>|```mermaid\n(.*?)\n```' -or '$1$2' "$file" \
  | awk -v d="$work" 'BEGIN{n=0;f=""} /^[0-9]+[:-]/{if(f)close(f);n++;f=d"/b"n".mmd";sub(/^[0-9]+[:-]/,"")} {print > f}'
shopt -s nullglob
blocks=("$work"/*.mmd)
[ "${#blocks[@]}" -gt 0 ] || { echo "mermaid-check: no mermaid blocks in $file" >&2; exit 2; }
fail=0
for b in "${blocks[@]}"; do
  if "$mmdc" -i "$b" -o "$work/$(basename "$b").svg" >/dev/null 2>"$work/err"; then
    echo "  ok   $(basename "$b")"
  else
    echo "  FAIL $(basename "$b"):"; sed 's/^/    /' "$work/err" >&2; fail=1
  fi
done
[ "$fail" -eq 0 ] && echo "mermaid-check: all ${#blocks[@]} block(s) valid" || { echo "mermaid-check: syntax errors above" >&2; exit 1; }

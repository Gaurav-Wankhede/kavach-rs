#!/usr/bin/env bash
# Insert exec_prompt: None into every MemoryEntry literal (idempotent).

set -euo pipefail
cd "$(dirname "$0")/.."

mapfile -t files < <(rg -rl --glob '*.rs' 'lane:\s*(None|Some)' crates/)
for f in "${files[@]}"; do
  perl -0pi -e 's/(^(\s*)lane: None,\n)(\s*occupied_by:)/$1$2exec_prompt: None,\n$3/mg' "$f"
done
echo "patched ${#files[@]} files"

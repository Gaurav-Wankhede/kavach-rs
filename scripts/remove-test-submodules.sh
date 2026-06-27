#!/bin/bash
set -euo pipefail

cd /Users/gauravwankhede/kavach-rs

for file in $(fd '_test\.rs$' crates/kavach-engine/src/); do
  awk '
    /^mod [a-z_]+;$/ {
      next
    }
    { print }
  ' "$file" > "$file.tmp" && mv "$file.tmp" "$file"
done

echo "Done"

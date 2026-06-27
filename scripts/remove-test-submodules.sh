#!/bin/bash
set -euo pipefail

cd /Users/gauravwankhede/kavach-rs

for file in crates/kavach-engine/src/gates/*_test.rs; do
  awk '
    /^mod [a-z_]+;$/ {
      next
    }
    { print }
  ' "$file" > "$file.tmp" && mv "$file.tmp" "$file"
done

echo "Done"

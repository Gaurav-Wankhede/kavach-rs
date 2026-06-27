#!/bin/bash
set -euo pipefail

cd /Users/gauravwankhede/kavach-rs

for file in $(rg -l 'mod tests;' crates/ --type rust); do
  awk '/^#\[cfg\(test\)\]$/ { if (prev ~ /^#\[path = /) { next } } { print; prev = $0 }' "$file" > "$file.tmp" && mv "$file.tmp" "$file"
done

echo "Done"

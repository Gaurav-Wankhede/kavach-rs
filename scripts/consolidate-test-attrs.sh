#!/bin/bash
set -euo pipefail

cd /Users/gauravwankhede/kavach-rs

# Remove all duplicate #[path] and #[cfg(test)] attributes, leaving just one of each
for file in $(rg -l 'mod tests;' crates/ --type rust); do
  # Remove duplicate #[path] lines
  awk '!seen[$0]++' "$file" > "$file.tmp" && mv "$file.tmp" "$file"

  # Remove duplicate #[cfg(test)] lines
  awk '!(/^#\[cfg\(test\)\]$/ && ++count > 1) {print}' "$file" > "$file.tmp" && mv "$file.tmp" "$file"
done

echo "Done"

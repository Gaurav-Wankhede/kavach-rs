#!/bin/bash
set -euo pipefail

cd /Users/gauravwankhede/kavach-rs

# Find all files with duplicate #[cfg(test)] or #[path] attributes near mod tests
for file in $(rg -l 'mod tests;' crates/ --type rust); do
  # Use awk to remove duplicate consecutive lines, but preserve non-consecutive ones
  awk '
    prev_line = ""
    {
      if ($0 != prev_line) {
        print $0
      }
      prev_line = $0
    }
  ' "$file" > "$file.tmp" && mv "$file.tmp" "$file"
done

echo "Done fixing duplicate attributes"

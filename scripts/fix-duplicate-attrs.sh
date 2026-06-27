#!/bin/bash
set -euo pipefail

cd /Users/gauravwankhede/kavach-rs

for file in $(rg -l 'mod tests;' crates/ --type rust); do
  # Find lines with #[cfg(test)] before mod tests, keep only first instance
  awk '
    in_test_block = 0
    cfg_count = 0
    path_count = 0

    /#\[cfg\(test\)\]/ {
      cfg_count++
      if (cfg_count > 1) {
        next
      }
    }
    /#\[path = / {
      path_count++
      if (path_count > 1) {
        next
      }
    }
    {
      print
    }
  ' "$file" > "$file.tmp" && mv "$file.tmp" "$file"
done

echo "Done"

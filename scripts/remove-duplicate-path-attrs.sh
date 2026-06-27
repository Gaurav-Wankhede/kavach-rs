#!/bin/bash
set -euo pipefail

cd /Users/gauravwankhede/kavach-rs

# Remove duplicate #[path] attributes from files with mod tests declarations
rg -l '#\[path = "' crates/ --type rust | while read -r file; do
  count=$(rg -c '#\[path = "' "$file")
  if [[ $count -gt 1 ]]; then
    # Get lines with #[path]
    second_path_line=$(rg -n '#\[path = "' "$file" | sed -n '2p' | cut -d: -f1)
    if [[ -n "$second_path_line" ]]; then
      sed -i '' "${second_path_line}d" "$file"
      echo "Removed duplicate #[path] at line $second_path_line in $file"
    fi
  fi
done

echo "Done"

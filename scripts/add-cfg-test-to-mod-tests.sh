#!/bin/bash
set -euo pipefail

cd /Users/gauravwankhede/kavach-rs

# Find all files with bare "mod tests;" without #[cfg(test)] and add it
rg -l '^\s*mod tests;\s*$' crates/ --type rust | while read -r file; do
  # Check if it already has #[cfg(test)]
  if rg -q '#\[cfg\(test\)\]' "$file"; then
    continue
  fi

  # Check if line before mod tests is already #[path = ...]
  if rg -B 1 '^\s*mod tests;\s*$' "$file" | rg -q '#\[path'; then
    # It has #[path] but not #[cfg(test)], need to add it
    sd '(#\[path = "[^"]+"\])\nmod tests;' '$1\n#[cfg(test)]\nmod tests;' "$file" || true
  else
    # Just add #[cfg(test)] before mod tests
    sd '^\s*mod tests;\s*$' '#[cfg(test)]\nmod tests;' "$file" || true
  fi
done

echo "Fix complete"

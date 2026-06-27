#!/bin/bash
set -euo pipefail

cd /Users/gauravwankhede/kavach-rs

for file in $(rg -l 'mod tests;' crates/ --type rust); do
  sd '#\[cfg\(test\)\]\n#\[path = "[^"]+"\]\n#\[cfg\(test\)\]' '#[cfg(test)]\n#[path = "[^"]+"' "$file" || true
done

echo "Done"

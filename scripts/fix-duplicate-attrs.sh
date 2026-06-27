#!/bin/bash
set -euo pipefail

cd /Users/gauravwankhede/kavach-rs

for file in $(rg -l 'mod tests;' crates/ --type rust); do
  perl -i -pe 's/#\[cfg\(test\)\]\n#\[path = "[^"]+"\]\n#\[cfg\(test\)\]/#[cfg(test)]\n#[path = "/' "$file"
done

echo "Done"

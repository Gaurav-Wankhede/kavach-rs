#!/bin/bash
set -euo pipefail

cd /Users/gauravwankhede/kavach-rs

for file in $(rg -l 'mod tests;' crates/ --type rust); do
  sed -i '' '/#\[cfg(test)\]$/{N;N;s/#\[cfg(test)\]\n#\[path = "[^"]*"\]\n#\[cfg(test)\]/#[cfg(test)]\n#[path = "TEST_NAME"/;};' "$file"
done

echo "Done"

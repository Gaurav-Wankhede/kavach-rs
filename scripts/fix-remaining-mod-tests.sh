#!/bin/bash
set -euo pipefail

cd /Users/gauravwankhede/kavach-rs

# Find all files with bare "mod tests;" and convert them to #[path = "{stem}_test.rs"]
rg -l '^\s*mod tests;\s*$' crates/ --type rust | while read -r file; do
  # Extract the stem (filename without .rs)
  stem=$(basename "$file" .rs)

  # Check if the corresponding {stem}_test.rs file exists
  dir=$(dirname "$file")
  test_file="${dir}/${stem}_test.rs"

  if [[ -f "$test_file" ]]; then
    # Convert bare mod tests; to #[path] version
    sd '^\s*#\[cfg\(test\)\]\s*\nmod tests;' "#[cfg(test)]\n#[path = \"${stem}_test.rs\"]\nmod tests;" "$file" || true
    sd '^\s*mod tests;\s*$' "#[cfg(test)]\n#[path = \"${stem}_test.rs\"]\nmod tests;" "$file" || true
  else
    echo "WARNING: No test file found for $file (expected ${test_file})"
  fi
done

echo "Fix complete"

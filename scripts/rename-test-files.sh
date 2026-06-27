#!/bin/bash
set -euo pipefail

cd /Users/gauravwankhede/kavach-rs

# Rename all tests.rs files to {stem}_test.rs and fix references
fd -g 'tests.rs' crates/ | while read -r file; do
  # Extract parent directory and stem
  dir=$(dirname "$file")
  parent_dir=$(dirname "$dir")
  stem=$(basename "$dir")
  target="${parent_dir}/${stem}_test.rs"

  # Guard against collision: if target exists, it's stale and must be removed
  if [[ -f "$target" ]]; then
    # Check if it's a stale duplicate by verifying the real tests are in the subdirectory
    if [[ -f "$file" ]]; then
      echo "Removing stale duplicate: $target"
      git rm "$target"
    else
      echo "COLLISION: $target (but source $file missing)"
      exit 1
    fi
  fi

  # Move the file
  git mv "$file" "$target"

  # Find the parent module file: either {stem}.rs or lib.rs/main.rs in parent_dir
  parent_module=""
  if [[ -f "${parent_dir}/${stem}.rs" ]]; then
    parent_module="${parent_dir}/${stem}.rs"
  elif [[ -f "${parent_dir}/lib.rs" ]]; then
    parent_module="${parent_dir}/lib.rs"
  elif [[ -f "${parent_dir}/main.rs" ]]; then
    parent_module="${parent_dir}/main.rs"
  fi

  if [[ -z "$parent_module" ]]; then
    echo "WARNING: Could not find parent module for $file"
    continue
  fi

  # Fix the path reference in the parent module
  if rg -q "#\[path = \"${stem}/tests\.rs\"\]" "$parent_module"; then
    sd "#\[path = \"${stem}/tests\.rs\"\]" "#[path = \"${stem}_test.rs\"]" "$parent_module"
  elif rg -q "mod tests" "$parent_module"; then
    # If there's a bare mod tests; without #[path], check if it references a tests dir
    if rg -q "${stem}/tests" "$parent_module"; then
      echo "WARNING: Found bare mod tests; referencing ${stem}/tests in $parent_module"
      # Try to fix if there's a pattern to match
      sd "${stem}/tests" "${stem}_test" "$parent_module" || true
    fi
  fi
done

echo "Rename complete"

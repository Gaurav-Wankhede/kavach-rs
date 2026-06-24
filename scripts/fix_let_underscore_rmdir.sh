#!/usr/bin/env bash
set -euo pipefail
rg -l 'let _ = (std::)?fs::remove_dir_all\(' crates -g '*.rs' \
  | while read -r f; do
      sd 'let _ = ((std::)?fs::remove_dir_all\([^)]*\));' 'drop($1);' "$f"
    done
echo "rewrote let-underscore remove_dir_all -> drop() across matched files"

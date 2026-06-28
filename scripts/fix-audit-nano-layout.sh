#!/usr/bin/env bash
# Convert the audit module from legacy mod.rs to the workspace's Rust-2024
# foo.rs + foo/ layout (clippy mod_module_files = deny), then regen docs/CLI.md.
set -euo pipefail
d="crates/kavach-cli/src/cmd/audit"
git mv "$d/lens/mod.rs" "$d/lens.rs"
git mv "$d/mod.rs" "$d.rs"
echo "renamed: audit/mod.rs -> audit.rs ; lens/mod.rs -> lens.rs"

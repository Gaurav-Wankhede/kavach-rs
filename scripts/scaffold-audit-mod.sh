#!/usr/bin/env bash
# Scaffolds the audit module declaration tree (mod-only files carry no testable
# logic — the sanctioned KAVACH_TDD_BYPASS carve-out for pure wiring). Logic files
# (finding/walk/lens/*) are written test-first via the normal gate, NOT here.
set -euo pipefail
root="crates/kavach-cli/src/cmd/audit"
mkdir -p "$root/lens"
# mod.rs — declarations + the run() entry stub is written separately test-first.
cat > "$root/lens/mod.rs" <<'RS'
//! Audit lenses — one detector family per file, all returning unified Findings.
pub(super) mod security;
pub(super) mod silent_fail;
pub(super) mod worst_practice;
pub(super) mod yagni;
RS
echo "scaffolded: $root/lens/mod.rs"

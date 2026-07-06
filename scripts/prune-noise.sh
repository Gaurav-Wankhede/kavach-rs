#!/usr/bin/env bash
# Remove verified-unreferenced cruft (git rm, current tree only — no history rewrite). Idempotent.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

# Unreferenced folders/files confirmed by reference-audit (rg over repo, README, justfile, code).
TARGETS=(
  supply-chain                       # cargo-vet config, CI-only, unreferenced
  assets                             # only a stale duplicate of transfer-package/CLAUDE.md
  ORPHANS.md                         # dated one-shot orphan census (2026-06-24)
  docs/architecture/kavach-lld.html  # stale generated diagram, not README-referenced
  .kavach/security-scan.json         # transient scan artifact
)
for t in "${TARGETS[@]}"; do
  [ -e "$t" ] && git rm -r --quiet "$t" || true
done

# Keep transient .kavach scan output out of git going forward.
grep -qxF '/.kavach/security-scan.json' .gitignore 2>/dev/null || printf '/.kavach/security-scan.json\n' >> .gitignore

echo "prune-noise: done. KEPT (load-bearing): transfer-package scripts docs/CLI.md .claude .cursor .kavach(settings)"

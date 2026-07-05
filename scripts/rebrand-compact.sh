#!/usr/bin/env bash
# Bulk rebrand injection-compaction to kavach-native naming (rnr files, sd symbols). Idempotent.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

TOON=crates/kavach-toon/src
CLI=crates/kavach-cli/src

[ -f "$TOON/caveman.rs" ] && rnr regex -f -s caveman compact "$TOON/caveman.rs" || true
[ -d "$TOON/caveman" ] && { rnr regex -f -s caveman compact "$TOON"/caveman/*.rs || true; mv "$TOON/caveman" "$TOON/compact"; }
[ -f crates/kavach-toon/tests/caveman_test.rs ] && mv crates/kavach-toon/tests/caveman_test.rs crates/kavach-toon/tests/compact_test.rs || true
[ -f "$CLI/cmd/caveman.rs" ] && mv "$CLI/cmd/caveman.rs" "$CLI/cmd/compact.rs" || true

FILES=$(rg -l -i 'caveman' --glob '!target' --glob '!docs/CLI.md' --glob '!scripts/rebrand-compact.sh' || true)
for f in $FILES; do
  sd 'CavemanError' 'CompactError' "$f"
  sd 'CAVEMAN_RECORDED' 'COMPACTION_RECORDED' "$f"
  sd 'caveman_inject' 'compact_inject' "$f"
  sd 'Caveman' 'Compact' "$f"
  sd 'caveman' 'compact' "$f"
done

echo "rebrand-compact: done. Verify: rg -i 'caveman|ponytail|polytail' --glob '!target' (expect zero)"

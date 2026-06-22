#!/usr/bin/env bash
# Mechanical wiring audit: flags DEFINED-but-never-CALLED feature surfaces.
# Read-only, rg-only. Orphans are CANDIDATES (rg is blind to trait dispatch /
# re-exports / dynamic verb strings) — verify the call path before acting.
# Rationale + sh-vs-llm tradeoff: decision.audit.sh-vs-llm.
set -uo pipefail
cd "$(git rev-parse --show-toplevel)" || exit 1

NOTEST=(-t rust -g'!*_test.rs' -g'!*/tests.rs' -g'!tests/**')
sum() { awk '{s+=$1} END{print s+0}'; }  # never pipe `rg -c` (path:N) through bc
hr() { printf '%s\n' '────────────────────────────────────────────────────'; }

echo "## 1. RPC VERBS — registered vs callers (engine+cli, tests excluded)"
hr
rg -o 'register_async_method\("([a-z0-9_.]+)"' -r '$1' crates/kavach-rpc/src/rpc.rs | sort -u |
while read -r verb; do
  n=$(rg "${NOTEST[@]}" -g'!crates/kavach-rpc/src/rpc.rs' --no-filename -c "\"$verb\"" crates/ 2>/dev/null | sum)
  [ "$n" -eq 0 ] && printf 'ORPHAN  %s\n' "$verb"
done
echo "(only orphan verbs listed)"

echo
echo "## 2. ENGINE GATES — kavach-patterns *guard* modules vs engine references"
hr
for f in crates/kavach-patterns/src/*guard*.rs; do
  [[ "$f" == *_test.rs ]] && continue
  mod=$(basename "$f" .rs)
  [[ "$mod" == *_tests ]] && continue
  n=$(rg "${NOTEST[@]}" --no-filename -c "${mod}::" crates/kavach-engine/src 2>/dev/null | sum)
  [ "$n" -eq 0 ] && printf 'ORPHAN  %s (engine never references)\n' "$mod"
done
echo "(empty == every gate wired)"

echo
echo "SCAN COMPLETE. Orphans are CANDIDATES — confirm the call path before acting."

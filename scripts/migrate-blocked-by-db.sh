#!/usr/bin/env bash
# Migrate DB rows off the retired blocker-prefix alias to DEPENDS_ON. Idempotent.
set -euo pipefail
cd "$(dirname "$0")/.."

D_KEY="decision.roadmap.depends-on-is-the-only-edge-no-parked-block"
D_TITLE="DEPENDS_ON is the only dependency verb; legacy blocker-prefix alias fully retired; a stale dep is resolved or the card removed, never parked blocked"
kavach db write --project kavach-rs --category decision --key "$D_KEY" --update-key "$D_KEY" --title "$D_TITLE" <<'EOF'
USER ARCHITECTURE + RESEARCH (topological scheduler, Kahn): a dependency edge ORDERS work, it never permanently BLOCKS. A card is runnable-now or ordered behind a DEPENDS_ON edge that is itself built; there is NO parked 'blocked' state. FINDINGS: (1) write path (cli + rpc mirror_depends_on_into_content) emits only DEPENDS_ON:. (2) The legacy blocker-style read-side parse alias is FULLY RETIRED 2026-06-30: removed from dep_key.rs::parse_declared_deps, which now accepts ONLY DEPENDS_ON: — do NOT reintroduce a blocker-prefix alias. (3) is_blocked (tasklist.rs) treats a deleted dep as satisfied (dangling=satisfied). THE TEETH: the all-blocked walk directive in user terms — a dep already DONE drops the edge; a STALE/obsolete dep means UPDATE the card to current version OR REMOVE it from todos (status-update verified / db delete), never leave it blocked; a CYCLE is cut. clean_exit REFUSES the stop on an all-blocked board (decision.engine.all-blocked-is-not-a-stop). SOURCE: https://en.wikipedia.org/wiki/Topological_sorting.
EOF

P_KEY="card-depends-on-line-must-be-clean-key-list"
P_TITLE="A DEPENDS_ON: card line must be ONLY <category>.<slug> keys on its own line, prose elsewhere"
kavach db write --project kavach-rs --category pattern --update-key "$P_KEY" --title "$P_TITLE" <<'EOF'
PATTERN (learned 2026-06-16; legacy blocker-prefix alias retired 2026-06-30): when authoring a kavach card with a dependency, put DEPENDS_ON: <key>[,<key>...] on its OWN line with NOTHING but real card keys after it; keep all prose (SCOPE/APPROACH) on SEPARATE lines below. DEPENDS_ON: is the ONLY accepted dependency prefix. A dep key is <category>.<slug> (decision/roadmap/research/pattern/app_spec). RATIONALE: the readiness parser turns each token after DEPENDS_ON: into a DAG edge; prose on that line = phantom missing nodes = the whole Continuous Loop wedges to ALL_BLOCKED. ALSO: depend on a ROADMAP card (which has a kanban status), never a decision/research row (no status -> never resolves).
EOF

echo "=== residual scan: any DB row still carrying the retired alias? ==="
residual=0
for p in backend heroic-video-agent kavach-global kavach-rs nicole-carpenter; do
  hits=$(kavach db search --project "$p" --contains "BLOCKED_BY:" 2>/dev/null | rg -c "BLOCKED_BY:" || true)
  if [ "${hits:-0}" != "0" ]; then
    echo "  $p: $hits row(s) still have a BLOCKED_BY: line — review/migrate"
    residual=$((residual + hits))
  fi
done
[ "$residual" = "0" ] && echo "migrate-blocked-by-db: clean — zero residual BLOCKED_BY: dependency lines."

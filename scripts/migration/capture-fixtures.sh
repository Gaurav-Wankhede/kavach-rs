#!/usr/bin/env bash
# capture-fixtures.sh — Phase 0 of the SurrealDB→Turso migration.
# Captures golden-master outputs of every read verb, per project, from the
# CURRENT (SurrealDB-backed) kavach binary. Later, the same capture run
# against the TursoStore build must reproduce these files exactly
# (modulo documented bm25 absolute-score deltas; ordering must match).
#
# Usage:
#   scripts/migration/capture-fixtures.sh                 # all projects
#   PROJECTS="gaurav-portfolio youtube" scripts/migration/capture-fixtures.sh
#
# Output: tests/fixtures/turso-parity/<project>/<verb>_<arg>.normalized.txt
set -u

KAVACH="${KAVACH:-kavach}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"          # repo root
OUT="$ROOT/tests/fixtures/turso-parity"
CATEGORIES=(decision research roadmap pattern app_spec)
SEARCH_PROBES=("rust" "test" "config")

mkdir -p "$OUT"

# Normalize volatile tokens so goldens compare equal across runs/engines:
#  - RFC3339 / unix timestamps        -> <TS>
#  - relative durations ("5m ago")    -> <AGO>
normalize() {
  sed -E \
    -e 's/[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9:.]+Z/<TS>/g' \
    -e 's/\b1[0-9]{9}\b/<EPOCH>/g' \
    -e 's/[0-9]+(ms|s|m|h|d) ago/<AGO>/g' \
    -e 's/duration_ms[^,}]*/duration_ms<N>/g'
}

projects=()
if [ -n "${PROJECTS:-}" ]; then
  read -ra projects <<< "$PROJECTS"
else
  while IFS= read -r line; do
    p="$(printf '%s' "$line" | sed -E 's/^([A-Za-z0-9_-]+) .*/\1/')"
    [ "$p" = "kavach-test-fix" ] && continue      # throwaway fixture project
    [ -n "$p" ] && projects+=("$p")
  done < <("$KAVACH" db list-projects 2>/dev/null)
fi

echo "capturing ${#projects[@]} project(s) -> $OUT"

for proj in "${projects[@]}"; do
  dir="$OUT/$proj"; mkdir -p "$dir"

  # 1) Full breadth query per category (titles) + deep dump per category.
  for cat in "${CATEGORIES[@]}"; do
    "$KAVACH" db query --project "$proj" --category "$cat" 2>&1 \
      | normalize > "$dir/query_${cat}_breadth.txt"
    "$KAVACH" db query --project "$proj" --category "$cat" --depth all 2>&1 \
      | normalize > "$dir/query_${cat}_deep.txt"
  done

  # 2) Search: unfiltered listing + fixed probes (BM25 ordering parity).
  "$KAVACH" db search --project "$proj" --limit 500 2>&1 \
    | normalize > "$dir/search_unfiltered.txt"
  for term in "${SEARCH_PROBES[@]}"; do
    "$KAVACH" db search --project "$proj" --contains "$term" --limit 100 2>&1 \
      | normalize > "$dir/search_contains_${term}.txt"
  done

  # 3) Kanban board state.
  "$KAVACH" db kanban --project "$proj" 2>&1 \
    | normalize > "$dir/kanban.txt"

  # 4) Per-entry get --full (keys harvested from the deep query dumps).
  n_get=0
  for cat in "${CATEGORIES[@]}"; do
    grep -oE '^\[?[A-Za-z0-9._:-]+\]?' "$dir/query_${cat}_deep.txt" 2>/dev/null \
      | tr -d '[]' | sort -u | head -50 | while IFS= read -r key; do
        [ -z "$key" ] && continue
        safe=$(printf '%s' "$key" | tr '/:' '__')
        "$KAVACH" db get --project "$proj" --category "$cat" --key "$key" --full 2>&1 \
          | normalize > "$dir/get_${cat}_${safe}.txt"
      done
  done

  echo "  ✓ $proj ($(ls "$dir" | wc -l | tr -d ' ') fixtures)"
done

# 5) Graph queries are global (not per-project).
gdir="$OUT/_global"; mkdir -p "$gdir"
"$KAVACH" db graph-query --limit 200 2>&1 | normalize > "$gdir/graph_all.txt"
for etype in skill rule gate project memory session concept event_type; do
  "$KAVACH" db graph-query --entity-type "$etype" --limit 100 2>&1 \
    | normalize > "$gdir/graph_type_${etype}.txt"
done
echo "  ✓ _global ($(ls "$gdir" | wc -l | tr -d ' ') fixtures)"
echo "done."

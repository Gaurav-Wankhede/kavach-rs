#!/usr/bin/env bash
# baseline-latency.sh — Phase 0: measure current gate overhead (SurrealDB-backed).
# Writes tests/fixtures/turso-parity/_latency-baseline.txt
# Re-run with the Turso build; PRD §5 gate: p95 <= baseline +10%.
set -u
KAVACH="${KAVACH:-kavach}"
OUT="$(cd "$(dirname "$0")/../.." && pwd)/tests/fixtures/turso-parity/_latency-baseline.txt"
N="${N:-50}"
PRE='{"session_id":"bench","cwd":"/tmp","hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"echo hi"}}'
INTENT='{"session_id":"bench","cwd":"/tmp","prompt":"benchmark probe for latency baseline"}'

now_ns() { date +%s%N; }

run_gate() { local gate="$1" payload="$2" times="$3" out="$4"
  local i start end ms
  : > "$out"
  for ((i=0;i<times;i++)); do
    start=$(now_ns)
    printf '%s' "$payload" | "$KAVACH" gates "$gate" --hook >/dev/null 2>&1
    end=$(now_ns)
    ms=$(( (end - start) / 1000000 ))
    echo "$ms" >> "$out"
  done
}

mkdir -p "$(dirname "$OUT")"
{
echo "# kavach gate latency baseline (SurrealDB backend), $(date '+%Y-%m-%d %H:%M:%S'), N=$N"
for spec in "pre-tool:$PRE" "intent:$INTENT"; do
  gate="${spec%%:*}"; payload="${spec#*:}"
  f="/tmp/kv_lat_${gate}.txt"
  run_gate "$gate" "$payload" "$N" "$f"
  sort -n "$f" | awk -v g="$gate" -v n="$N" '
    {v[NR]=$1}
    END{
      p50=v[int(n*0.5+0.5)]; p95=v[int(n*0.95+0.5)]; p99=v[int(n*0.99+0.5)];
      printf "%s_ms p50=%d p95=%d p99=%d min=%d max=%d\n", g,p50,p95,p99,v[1],v[n]
    }'
done
} | tee "$OUT"

#!/usr/bin/env bash
# Runtime prober: actually INVOKES each read-only CLI command + each hook gate,
# captures real stdout bytes + exit code, classifies EMPTY / ERROR / OK.
# Proves response-presence, not just source wiring. See decision.audit.runtime-probe.
set -uo pipefail
BIN="${KAVACH_BIN:-$HOME/.local/bin/kavach}"
P=kavach-rs
probe() { # label  cmd...
  local label="$1"; shift
  local out rc
  out="$("$@" 2>&1)"; rc=$?
  local n=${#out}
  local verdict
  if [ "$rc" -ne 0 ] && [ "$n" -eq 0 ]; then verdict="ERROR+EMPTY (rc=$rc)"
  elif [ "$rc" -ne 0 ]; then verdict="ERROR rc=$rc (${n}b)"
  elif [ "$n" -eq 0 ]; then verdict="EMPTY (rc=0, 0 bytes)"
  else verdict="OK (${n}b)"; fi
  printf '%-34s %s\n' "$label" "$verdict"
}

echo "## A. READ-ONLY CLI COMMANDS (real invocation, stdout+exit captured)"
echo "──────────────────────────────────────────────────────────"
probe "status"            "$BIN" status
probe "context"           "$BIN" context --project "$P"
probe "db kanban"         "$BIN" db kanban --project "$P"
probe "db query"          "$BIN" db query --project "$P"
probe "db list-projects"  "$BIN" db list-projects
probe "db query-raw"      "$BIN" db query-raw --query "INFO FOR DB"
probe "phase status"      "$BIN" phase status
probe "tasks"             "$BIN" tasks --project "$P"
probe "todos"             "$BIN" todos --project "$P"
probe "mistake top"       "$BIN" mistake top
probe "think"             "$BIN" think --project "$P" "wiring audit"
probe "bg status"         "$BIN" bg status --project "$P"
probe "goal status"       "$BIN" goal status --project "$P"
probe "toolbelt"          "$BIN" toolbelt
probe "doctor"            "$BIN" doctor
probe "servers"           "$BIN" servers
probe "spec"              "$BIN" spec --project "$P"
probe "security"          "$BIN" security --project "$P"
probe "loophole status"   "$BIN" loophole status --project "$P"
probe "loop status"       "$BIN" loop status --project "$P"

echo
echo "## B. HOOK GATES (synthetic hook JSON on stdin — non-blocking probe)"
echo "──────────────────────────────────────────────────────────"
# minimal valid hook payloads per event family; gates must EXIT cleanly + may emit context
HOOK_BASE='{"session_id":"audit-probe","cwd":"'"$PWD"'","hook_event_name":"X"}'
probe_gate() { # gate  json
  local gate="$1" json="$2" out rc
  out="$(printf '%s' "$json" | "$BIN" gates "$gate" --hook 2>&1)"; rc=$?
  local n=${#out}
  printf '%-26s exit=%-3s bytes=%-5s %s\n' "$gate" "$rc" "$n" \
    "$([ "$rc" -le 2 ] && echo 'FIRES (clean exit code)' || echo "ABNORMAL rc=$rc")"
}
probe_gate session-start  "$HOOK_BASE"
probe_gate intent         '{"session_id":"audit-probe","cwd":"'"$PWD"'","prompt":"audit the gates"}'
probe_gate pre-tool       '{"session_id":"audit-probe","cwd":"'"$PWD"'","tool_name":"Bash","tool_input":{"command":"ls"}}'
probe_gate pre-write      '{"session_id":"audit-probe","cwd":"'"$PWD"'","tool_name":"Write","tool_input":{"file_path":"/tmp/x.rs","content":"fn main(){}"}}'
probe_gate post-write     '{"session_id":"audit-probe","cwd":"'"$PWD"'","tool_name":"Write","tool_input":{"file_path":"/tmp/x.rs"}}'
probe_gate post-tool      '{"session_id":"audit-probe","cwd":"'"$PWD"'","tool_name":"Bash","tool_input":{}}'
probe_gate stop           "$HOOK_BASE"
probe_gate pre-compact    "$HOOK_BASE"
probe_gate post-compact   "$HOOK_BASE"
probe_gate session-end    "$HOOK_BASE"
probe_gate subagent-stop  "$HOOK_BASE"

echo
echo "PROBE COMPLETE. EMPTY rc=0 = command runs but returns nothing (investigate)."
echo "ERROR+EMPTY = the silent-failure class. Gate abnormal rc (>2) = hook misfire."

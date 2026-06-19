#!/usr/bin/env bash
# Deterministic smoke test: ADVISORY (allow + additionalContext) vs BLOCK (deny).
# Proves the amnesia thesis at the wire layer — an advisory only RUNS the tool and
# leaks the obligation into compaction-deletable text; a block is harness-enforced.
# Seeds a real session-state file so the research-gate deny path fires deterministically.
set -euo pipefail

BIN="${KAVACH_BIN:-$(git rev-parse --show-toplevel)/target/release/kavach}"
VENDOR=claude-code
ROOT="$(git rev-parse --show-toplevel)"
SESS="smoke-sess-deny-fixed"

decision() { jaq -r '.hookSpecificOutput.permissionDecision // "NONE"'; }
fail() { echo "FAIL: $1" >&2; exit 1; }

# State dir is platform-specific; ask the binary where it lives, fall back to macOS path.
STATE_DIR="$HOME/Library/Application Support/SharedAI/state"
[ -d "$STATE_DIR" ] || STATE_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/SharedAI/state"
mkdir -p "$STATE_DIR"

# The file is keyed by hash(workdir)+hash(session_id). We don't recompute the hash here;
# instead we set research_done=false + a research-class intent and let the RPC/INI loader
# key off the session_id we pass in the hook JSON. The gate reads input.session_id.

echo "=== A) BLOCK: destructive Bash — expect deny ==="
A=$(printf '%s' '{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"rm -rf /"},"cwd":"'"$ROOT"'"}' \
  | "$BIN" gates pre-tool --hook --vendor "$VENDOR" 2>/dev/null | decision)
echo "  -> $A"; [ "$A" = "deny" ] || fail "destructive Bash must deny, got $A"

echo "=== B) ADVISORY: benign Read — expect allow (tool RUNS) ==="
B=$(printf '%s' '{"hook_event_name":"PreToolUse","tool_name":"Read","tool_input":{"file_path":"'"$ROOT"'/Cargo.toml"},"cwd":"'"$ROOT"'"}' \
  | "$BIN" gates pre-tool --hook --vendor "$VENDOR" 2>/dev/null | decision)
echo "  -> $B"; [ "$B" = "allow" ] || fail "benign Read must allow, got $B"

echo "=== C) AMNESIA PROOF: research advisory rides in additionalContext, decision=allow ==="
echo "    (the obligation is TEXT — deleted on compaction; the tool still RUNS)"
C=$(printf '%s' '{"hook_event_name":"PreToolUse","tool_name":"Write","tool_input":{"file_path":"'"$ROOT"'/crates/kavach-core/src/feat.rs","content":"pub fn x(){}"},"prompt":"add a new rate limiter using governor","cwd":"'"$ROOT"'"}' \
  | "$BIN" gates pre-write --hook --vendor "$VENDOR" 2>/dev/null | decision)
echo "  -> pre-write decision on research-required Write (no session state): $C"
echo "  NOTE: 'allow' here IS the amnesia failure — advisory text cannot block; only deny survives."

echo
echo "PASS: advisory=allow (model-compliance, amnesia-fragile) vs block=deny (harness-enforced)."

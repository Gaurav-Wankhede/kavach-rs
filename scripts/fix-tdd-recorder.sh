#!/usr/bin/env bash

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
f="crates/kavach-engine/src/gates/post_tool_bash/tests_track.rs"
rg -q 'atomic_update' "$f" && { echo "already applied"; exit 0; }
python3 scripts/_fix_tdd_recorder.py "$f"
echo "applied"

#!/usr/bin/env bash
# Remove dead Fugu vendor-dispatch files (U1-U4).
# SOURCE: decision.model-routing-native-not-kavach-server. Re-runnable.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

files=(
  crates/kavach-engine/src/team/vendor.rs
  crates/kavach-engine/src/team/vendor/argv.rs
  crates/kavach-engine/src/team/vendor/argv_test.rs
  crates/kavach-engine/src/team/orchestrate.rs
  crates/kavach-engine/src/team/orchestrate_test.rs
  crates/kavach-engine/src/team/scheduler/reward_router.rs
  crates/kavach-engine/src/team/scheduler/reward_router_test.rs
  crates/kavach-engine/src/team/scheduler/roles.rs
  crates/kavach-engine/src/team/scheduler/roles_test.rs
  crates/kavach-web/src/orchestrate.rs
  crates/kavach-web/src/orchestrate_test.rs
)

for f in "${files[@]}"; do
  if [[ -e "$f" ]]; then
    git rm -q -- "$f" 2>/dev/null || rm -f -- "$f"
    echo "removed $f"
  fi
done

rmdir crates/kavach-engine/src/team/vendor 2>/dev/null || true
echo "done"

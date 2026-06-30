#!/usr/bin/env bash
# Retire BLOCKED_BY: for canonical DEPENDS_ON:, idempotent.
set -euo pipefail
cd "$(dirname "$0")/.."

RPC="crates/kavach-rpc/src/methods"
CLI="crates/kavach-cli/src/cmd/db"

sd -s -- '        let header = line
            .strip_prefix("BLOCKED_BY:")
            .or_else(|| line.strip_prefix("DEPENDS_ON:"));' \
        '        let header = line.strip_prefix("DEPENDS_ON:");' \
  "$RPC/roadmap/readiness/dep_key.rs"

for f in "$RPC/db/write.rs" "$CLI/write.rs"; do
  sd -s -- '.filter(|l| l.starts_with("DEPENDS_ON:") || l.starts_with("BLOCKED_BY:"))' \
          '.filter(|l| l.starts_with("DEPENDS_ON:"))' \
    "$f"
done

while IFS= read -r f; do
  sd -s -- 'BLOCKED_BY:' 'DEPENDS_ON:' "$f"
done < <(rg -l --fixed-strings 'BLOCKED_BY:' crates/ || true)

while IFS= read -r f; do
  sd -s -- '`DEPENDS_ON:`/`DEPENDS_ON:`' '`DEPENDS_ON:`' "$f"
  sd -s -- '`DEPENDS_ON:` / `DEPENDS_ON:`' '`DEPENDS_ON:`' "$f"
  sd -s -- 'DEPENDS_ON:/DEPENDS_ON:' 'DEPENDS_ON:' "$f"
done < <(rg -l --fixed-strings 'DEPENDS_ON:' crates/)

sd -s -- 'starts with `DEPENDS_ON:` or
/// `DEPENDS_ON:`,' \
        'starts with `DEPENDS_ON:`,' \
  "$RPC/roadmap/readiness/dep_key.rs"
sd -s -- 'Parse `DEPENDS_ON:` / `DEPENDS_ON:` declarations' \
        'Parse `DEPENDS_ON:` declarations' \
  "$RPC/roadmap/readiness/dep_key.rs"

echo "rename-blocked-by: done. Remaining BLOCKED_BY tokens:"
rg -c --fixed-strings 'BLOCKED_BY' crates/ || echo "  0 — fully retired."

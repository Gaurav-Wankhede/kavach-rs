#!/usr/bin/env bash
# Retire 7 orphan RPC verbs. SEE decision.harness.orphan-rpc-retire-2026-06-24.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
RPC=crates/kavach-rpc/src
M="$RPC/methods"

# 1. rpc.rs — drop the 7 register_async_method blocks (each: `module\n .register..("verb"..)..register verb: {e}"))?;`).
#    Anchor on the unique trailing map_err line and the leading `module` via a multiline ast-grep on the chain stmt.
for verb in \
  "system.schema_apply" "projects.list_all" "concept.find" \
  "nlm-NONE" \
  "replay.event" "replay.trajectory" "trust.should_surface" "bulk.sweep_get"; do
  [ "$verb" = "nlm-NONE" ] && continue
  # Structural delete: the whole `module .register_async_method("<verb>", ...) ... .map_err(...register <verb>: {e}"))?;`
  ast-grep run --update-all \
    --pattern "module
        .register_async_method(\"$verb\", \$\$\$)
        .map_err(|e| internal(format!(\"register $verb: {e}\")))?;" \
    --rewrite "" "$RPC/rpc.rs" 2>/dev/null || true
done

# 2. replay module is wholly dead (both verbs retired) — delete file + mod decl.
rm -f "$M/replay.rs"
sd '(?m)^pub mod replay;\n' '' "$M/methods.rs" 2>/dev/null || sd '(?m)^pub mod replay;\n' '' "$RPC/methods.rs"

# 3. bulk/get.rs — strip dead `get()` fn + GetParams struct; KEEP GetResult (reused by list.rs).
ast-grep run --update-all \
  --pattern 'pub async fn get(state: &AppState, p: GetParams) -> Result<Option<GetResult>, ErrorObjectOwned> { $$$ }' \
  --rewrite '' "$M/bulk/get.rs"
ast-grep run --update-all \
  --pattern 'pub struct GetParams { $$$ }' \
  --rewrite '' "$M/bulk/get.rs"
# its derive attr + doc lines orphan above the removed struct — drop the now-danging GetParams derive/comment block
sd '(?s)#\[derive\(Debug, Serialize, Deserialize\)\]\n#\[non_exhaustive\]\n\n' '' "$M/bulk/get.rs" 2>/dev/null || true
# narrow bulk.rs re-export: GetParams + get are gone, GetResult stays
sd 'pub use get::\{GetParams, GetResult, get\};' 'pub use get::GetResult;' "$M/bulk.rs"

# 4. system.rs — strip schema_apply fn + its now-unused apply_schema import (health does not use it).
ast-grep run --update-all \
  --pattern 'pub async fn schema_apply(state: &AppState) -> Result<&'\''static str, ErrorObjectOwned> { $$$ }' \
  --rewrite '' "$M/system.rs"
sd '(?m)^use kavach_surreal::apply_schema;\n' '' "$M/system.rs"
# the schema_apply doc-comment block orphans above the removed fn
sd '(?s)/// Apply the system schema to the database\.\n///\n/// # Errors\n/// Returns an error if the schema application fails\.\n' '' "$M/system.rs" 2>/dev/null || true

# 5. projects.rs — strip list_all fn + its doc.
ast-grep run --update-all \
  --pattern 'pub async fn list_all(state: &AppState) -> Result<Vec<Project>, ErrorObjectOwned> { $$$ }' \
  --rewrite '' "$M/projects.rs"

# 6. concept.rs — strip find fn + FindParams + the now-unused graph_find_concept import token.
ast-grep run --update-all \
  --pattern 'pub async fn find(state: &AppState, p: FindParams) -> Result<Option<Entity>, ErrorObjectOwned> { $$$ }' \
  --rewrite '' "$M/concept.rs"
ast-grep run --update-all \
  --pattern 'pub struct FindParams { $$$ }' \
  --rewrite '' "$M/concept.rs"
sd 'graph_find_concept, ' '' "$M/concept.rs"

# 7. trust.rs — strip should_surface fn + ShouldSurfaceParams + ShouldSurfaceResult (classify kept; shares imports).
ast-grep run --update-all \
  --pattern 'pub async fn should_surface($$$) -> Result<ShouldSurfaceResult, ErrorObjectOwned> { $$$ }' \
  --rewrite '' "$M/trust.rs"
ast-grep run --update-all --pattern 'pub struct ShouldSurfaceParams { $$$ }' --rewrite '' "$M/trust.rs"
ast-grep run --update-all --pattern 'pub struct ShouldSurfaceResult { $$$ }' --rewrite '' "$M/trust.rs"

echo "retire-orphans: edits applied. Run 'cargo fmt -p kavach-rpc' then 'cargo check -p kavach-rpc'."

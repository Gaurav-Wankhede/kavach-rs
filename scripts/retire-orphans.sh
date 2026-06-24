#!/usr/bin/env bash
# Retire 7 orphan RPC verbs. SEE decision.harness.orphan-rpc-retire-2026-06-24.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
RPC=crates/kavach-rpc/src
M="$RPC/methods"

# 1. rpc.rs — drop the 7 register blocks.
for verb in \
  "system.schema_apply" "projects.list_all" "concept.find" \
  "replay.event" "replay.trajectory" "trust.should_surface" "bulk.sweep_get"; do
  ast-grep run --update-all \
    --pattern "module
        .register_async_method(\"$verb\", \$\$\$)
        .map_err(|e| internal(format!(\"register $verb: {e}\")))?;" \
    --rewrite "" "$RPC/rpc.rs" 2>/dev/null || true
done

# 2. replay — whole module dead.
rm -f "$M/replay.rs"
sd '(?m)^pub mod replay;\n' '' "$M/methods.rs" 2>/dev/null || sd '(?m)^pub mod replay;\n' '' "$RPC/methods.rs"

# 3. bulk/get.rs — drop get()+GetParams, keep GetResult (reused by list.rs).
ast-grep run --update-all \
  --pattern 'pub async fn get(state: &AppState, p: GetParams) -> Result<Option<GetResult>, ErrorObjectOwned> { $$$ }' \
  --rewrite '' "$M/bulk/get.rs"
ast-grep run --update-all \
  --pattern 'pub struct GetParams { $$$ }' \
  --rewrite '' "$M/bulk/get.rs"
sd '(?s)#\[derive\(Debug, Serialize, Deserialize\)\]\n#\[non_exhaustive\]\n\n' '' "$M/bulk/get.rs" 2>/dev/null || true
sd 'pub use get::\{GetParams, GetResult, get\};' 'pub use get::GetResult;' "$M/bulk.rs"

# 4. system.rs — drop schema_apply + its import.
ast-grep run --update-all \
  --pattern 'pub async fn schema_apply(state: &AppState) -> Result<&'\''static str, ErrorObjectOwned> { $$$ }' \
  --rewrite '' "$M/system.rs"
sd '(?m)^use kavach_surreal::apply_schema;\n' '' "$M/system.rs"
sd '(?s)/// Apply the system schema to the database\.\n///\n/// # Errors\n/// Returns an error if the schema application fails\.\n' '' "$M/system.rs" 2>/dev/null || true

# 5. projects.rs — drop list_all.
ast-grep run --update-all \
  --pattern 'pub async fn list_all(state: &AppState) -> Result<Vec<Project>, ErrorObjectOwned> { $$$ }' \
  --rewrite '' "$M/projects.rs"

# 6. concept.rs — drop find + FindParams + import token.
ast-grep run --update-all \
  --pattern 'pub async fn find(state: &AppState, p: FindParams) -> Result<Option<Entity>, ErrorObjectOwned> { $$$ }' \
  --rewrite '' "$M/concept.rs"
ast-grep run --update-all \
  --pattern 'pub struct FindParams { $$$ }' \
  --rewrite '' "$M/concept.rs"
sd 'graph_find_concept, ' '' "$M/concept.rs"

# 7. trust.rs — drop should_surface + its 2 DTOs (classify kept).
ast-grep run --update-all \
  --pattern 'pub async fn should_surface($$$) -> Result<ShouldSurfaceResult, ErrorObjectOwned> { $$$ }' \
  --rewrite '' "$M/trust.rs"
ast-grep run --update-all --pattern 'pub struct ShouldSurfaceParams { $$$ }' --rewrite '' "$M/trust.rs"
ast-grep run --update-all --pattern 'pub struct ShouldSurfaceResult { $$$ }' --rewrite '' "$M/trust.rs"

echo "retire-orphans: edits applied. Run 'cargo fmt -p kavach-rpc' then 'cargo check -p kavach-rpc'."

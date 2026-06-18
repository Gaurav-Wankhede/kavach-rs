// TIME: O(n) projects | SPACE: O(n)
// YEAR: 2026 | SEARCHED: 2026-06
//! `db.tree` RPC method — the project hierarchy forest.
//!
//! Read routed through the single-writer daemon. The recursive `TreeNode` DTO
//! mirrors `kavach_surreal::ProjectNode` but flattens display fields so the CLI
//! renders identically over RPC and the direct fallback.
//! SOURCE: <https://github.com/facebook/rocksdb/issues/1780>

use crate::error::internal;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};

/// No request fields — `tree` returns the whole forest.
///
/// A unit struct (serializes to `null`) matches the typed-params call shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct TreeParams;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct TreeResult {
    pub forest: Vec<TreeNode>,
}

/// One node in the project hierarchy; `children` recurses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct TreeNode {
    pub slug: String,
    pub workdir: Option<String>,
    pub children: Vec<Self>,
}

/// # Errors
/// Returns an RPC error when the forest read fails.
pub async fn tree(ctx: &AppState, _params: TreeParams) -> Result<TreeResult, ErrorObjectOwned> {
    let forest = kavach_surreal::projects_build_forest(&ctx.db)
        .await
        .map_err(|e| internal(e.to_string()))?
        .iter()
        .map(flatten)
        .collect();
    Ok(TreeResult { forest })
}

fn flatten(node: &kavach_surreal::ProjectNode) -> TreeNode {
    TreeNode {
        slug: node.project.slug.clone(),
        workdir: node.project.workdir.clone(),
        children: node.children.iter().map(flatten).collect(),
    }
}

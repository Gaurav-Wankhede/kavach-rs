// ALGO: Upsert
// TIME: O(1) avg | SPACE: O(1)
//! db.write RPC method — create or update entry.

use super::util::resolve_project_id;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_types::Priority;
use serde::{Deserialize, Serialize};

mod relationships;

const ERR_BOTH: &str = "'new' and 'update_key' are mutually exclusive";
const ERR_NEITHER: &str = "must specify 'new: true' or 'update_key'";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct WriteParams {
    pub project: String,
    pub category: String,
    pub key: String,
    pub title: String,
    pub content: Option<String>,
    #[serde(default)]
    pub new: Option<bool>,
    pub update_key: Option<String>,
    #[serde(default)]
    pub priority: Option<i64>,
    /// Fully-resolved inter-entry edges `(rel, target_qname)` the CLI extracted
    /// from body (frontmatter/wikilink/NLU) merged with `--depends-on`. The CLI
    /// owns extraction (it depends on `kavach-engine`; the daemon cannot — that
    /// would cycle); the daemon — the single `RocksDB` writer — owns projection.
    #[serde(default)]
    pub relationships: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct WriteResult {
    pub success: bool,
    pub id: Option<String>,
    pub error: Option<String>,
}

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when validation fails or database write fails.
pub async fn write(ctx: &AppState, params: WriteParams) -> Result<WriteResult, ErrorObjectOwned> {
    let is_new = params.new.unwrap_or_default();
    if !is_new && params.update_key.is_none() {
        return Ok(WriteResult {
            success: false,
            id: None,
            error: Some(ERR_NEITHER.to_owned()),
        });
    }
    if is_new && params.update_key.is_some() {
        return Ok(WriteResult {
            success: false,
            id: None,
            error: Some(ERR_BOTH.to_owned()),
        });
    }

    let pid = resolve_project_id(&ctx.db, &params.project).await?;
    let content = params.content.as_deref().unwrap_or("");

    let qname = format!("{}/{}/{}", params.project, params.category, params.key);
    let refs: Vec<String> = Vec::new();
    let priority = params.priority.map(Priority::new);
    let result = kavach_surreal::upsert_entry_full()
        .db(&ctx.db)
        .category(&params.category)
        .project_id(&pid)
        .entry_key(&params.key)
        .title(&params.title)
        .content(content)
        .event_source("rpc")
        .qualified_name(&qname)
        .references(&refs)
        .maybe_priority(priority)
        .build_for_call()
        .await;

    match result {
        Ok(id) => {
            relationships::project_relationships(ctx, &params, &qname).await;
            Ok(WriteResult {
                success: true,
                id: Some(format!("{id:?}")),
                error: None,
            })
        }
        Err(e) => Ok(WriteResult {
            success: false,
            id: None,
            error: Some(e.to_string()),
        }),
    }
}

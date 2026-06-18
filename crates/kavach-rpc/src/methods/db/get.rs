// TIME: O(1) avg | SPACE: O(1)
//! db.get RPC method — fetch single entry by key.

use super::util::resolve_project_id;
use crate::error::internal;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};

const STATUS_TODO: &str = "todo";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct GetParams {
    pub project: String,
    pub category: String,
    pub key: String,
    #[serde(default)]
    pub full: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct GetResult {
    pub found: bool,
    pub entry: Option<GetEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "RPC DTO at boundary")]
pub struct GetEntry {
    pub key: String,
    pub title: String,
    pub category: String,
    pub status: String,
    pub content: Option<String>,
    pub access_count: i64,
}

/// # Errors
/// Returns an RPC `ErrorObjectOwned` when the underlying `SurrealDB` query fails.
pub async fn get(ctx: &AppState, params: GetParams) -> Result<GetResult, ErrorObjectOwned> {
    let pid = resolve_project_id(&ctx.db, &params.project).await?;
    let entry = kavach_surreal::read::get_by_key(&ctx.db, &params.category, &pid, &params.key)
        .await
        .map_err(|e| internal(e.to_string()))?;

    match entry {
        Some(e) => {
            let include_content = params.full.unwrap_or_default();
            Ok(GetResult {
                found: true,
                entry: Some(GetEntry {
                    key: e.entry_key.clone(),
                    title: e.title.clone(),
                    category: e.category_str().to_owned(),
                    status: if e.entry_status_str().is_empty() {
                        STATUS_TODO.to_owned()
                    } else {
                        e.entry_status_str().to_owned()
                    },
                    content: include_content.then(|| e.content.clone()),
                    access_count: e.access_count.unwrap_or_default(),
                }),
            })
        }
        None => Ok(GetResult {
            found: false,
            entry: None,
        }),
    }
}

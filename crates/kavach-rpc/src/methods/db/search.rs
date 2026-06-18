// TIME: O(t·n log n) | SPACE: O(n)
// YEAR: 2026 | SEARCHED: 2026-06
//! `db.search` RPC method — filtered multi-table entry search.
//!
//! Read routed through the single-writer daemon. The `FilterBuilder` lives in
//! `kavach-surreal` (daemon-visible), so the daemon builds the filter from raw
//! `status`/`since`/`contains` params — the CLI passes strings, not a built expr.
//! SOURCE: <https://github.com/facebook/rocksdb/issues/1780>

use super::util::resolve_project_id;
use crate::error::internal;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::{FilterBuilder, FilterExpr};

mod types;

pub use types::{SearchHit, SearchParams, SearchResult};

const ALL_CATEGORIES: &[&str] = &["decision", "research", "roadmap", "pattern", "app_spec"];
const ALLOWED_STATUSES: &[&str] = &["todo", "in_progress", "done", "verified"];

/// # Errors
/// Returns an RPC error when the project is unknown or a table read fails.
pub async fn search(
    ctx: &AppState,
    params: SearchParams,
) -> Result<SearchResult, ErrorObjectOwned> {
    let pid = resolve_project_id(&ctx.db, &params.project).await?;
    let filter = build_filter(
        params.status.as_deref(),
        params.since.as_deref(),
        params.contains.as_deref(),
    );
    let tables: Vec<&str> = params
        .category
        .as_deref()
        .map_or_else(|| ALL_CATEGORIES.to_vec(), |c| vec![c]);

    let mut hits = Vec::new();
    for table in &tables {
        let rows = kavach_surreal::list_with_filter(
            &ctx.db,
            table,
            &pid,
            filter.as_ref(),
            Some(params.limit),
        )
        .await
        .map_err(|e| internal(format!("reading {table}: {e}")))?;
        for e in rows {
            let status = if e.entry_status_str().is_empty() {
                e.status_str()
            } else {
                e.entry_status_str()
            };
            hits.push((
                e.updated_at,
                SearchHit {
                    category: e.category_str().to_owned(),
                    key: e.entry_key.clone(),
                    title: e.title.clone(),
                    status: status.to_owned(),
                },
            ));
        }
    }
    hits.sort_by_key(|(ts, _)| std::cmp::Reverse(*ts));
    let entries = hits
        .into_iter()
        .take(params.limit)
        .map(|(_, h)| h)
        .collect();
    Ok(SearchResult { entries })
}

fn build_filter(
    status: Option<&str>,
    since: Option<&str>,
    contains: Option<&str>,
) -> Option<FilterExpr> {
    let mut b = FilterBuilder::new();
    if let Some(s) = status
        && ALLOWED_STATUSES.contains(&s)
    {
        b = b.eq("entry_status", s);
    }
    if let Some(d) = since
        && is_valid_duration(d)
    {
        b = b.since("updated_at", d);
    }
    if let Some(c) = contains {
        b = b.contains("title", c);
    }
    b.build()
}

fn is_valid_duration(s: &str) -> bool {
    if s.is_empty() || s.len() > 16 {
        return false;
    }
    let Some(&last) = s.as_bytes().last() else {
        return false;
    };
    if !matches!(last, b'd' | b'h' | b'm' | b's' | b'w' | b'y') {
        return false;
    }
    let Some(digits) = s.get(..s.len().saturating_sub(1)) else {
        return false;
    };
    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
}

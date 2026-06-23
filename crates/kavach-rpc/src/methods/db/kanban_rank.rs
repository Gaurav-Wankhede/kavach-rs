//! `db.kanban_ranked` — relevance-ranked runnable roadmap cards.
//!
//! Reuses the EXISTING recommender (`brain.think` = BM25-FTS ⊕ graph → RRF) to
//! reorder the runnable (todo/in_progress) kanban slice by relevance to the task
//! prompt, so the harness injects the cards that MATTER NOW, not the whole board
//! by priority. Empty prompt (session-start) ⇒ priority order kept. Fail-soft:
//! brain error ⇒ priority order; project missing ⇒ empty. See
//! decision.harness.dynamic-relevance-injection.
use crate::error::internal;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use serde::{Deserialize, Serialize};

use super::kanban::{KanbanParams, kanban};

/// Default top-K ranked cards to return (token discipline).
const DEFAULT_LIMIT: usize = 6;

/// Parameters for `db.kanban_ranked`.
#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct KanbanRankedParams {
    /// Project slug whose runnable cards to rank.
    pub project: String,
    /// Task prompt to rank against; empty ⇒ priority order (session-start).
    #[serde(default)]
    pub prompt: String,
    /// Max ranked cards to return; defaults to 6.
    #[serde(default)]
    pub limit: Option<usize>,
}

/// One runnable card in the ranked result (key + title + status only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC result DTO constructed at handler boundary"
)]
pub struct RankableCard {
    /// Roadmap entry key.
    pub key: String,
    /// Display title.
    pub title: String,
    /// `todo` or `in_progress` (runnable statuses only).
    pub status: String,
}

/// Result of `db.kanban_ranked`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC result DTO constructed at handler boundary"
)]
pub struct KanbanRankedResult {
    /// Relevance-ranked (or priority-ordered when prompt empty) runnable cards.
    pub cards: Vec<RankableCard>,
}

/// The two dispatchable statuses — the only work queue.
fn is_runnable(status: &str) -> bool {
    status == "todo" || status == "in_progress"
}

/// Reorder `cards` so those matched by `hit_ids` (brain.think recall, most
/// relevant first) lead in hit order, then the rest keep their input (priority)
/// order; truncate to `limit`. Empty `hit_ids` ⇒ input order preserved.
///
/// A hit id matches a card when it equals the key, or ends in `/<key>` or
/// `.<key>` (brain emits bare keys, `roadmap.<key>`, and `<proj>/roadmap/<key>`).
#[must_use]
pub fn rank_cards_by_relevance(
    cards: Vec<RankableCard>,
    hit_ids: &[String],
    limit: usize,
) -> Vec<RankableCard> {
    let matches = |id: &str, key: &str| {
        id == key || id.ends_with(&format!("/{key}")) || id.ends_with(&format!(".{key}"))
    };
    // Rank position of a card = index of the first hit that matches it; un-hit
    // cards get usize::MAX so they sort after all hits while keeping input order
    // among themselves (stable sort).
    let mut indexed: Vec<(usize, RankableCard)> = cards
        .into_iter()
        .map(|c| {
            let rank = hit_ids
                .iter()
                .position(|id| matches(id, &c.key))
                .unwrap_or(usize::MAX);
            (rank, c)
        })
        .collect();
    indexed.sort_by_key(|(rank, _)| *rank);
    indexed
        .into_iter()
        .map(|(_, c)| c)
        .take(limit)
        .collect()
}

/// Rank the runnable roadmap cards of a project by relevance to the prompt.
///
/// # Errors
/// Returns `ErrorObjectOwned` when the project is missing or the kanban read
/// fails. A brain.think error is swallowed (fail-soft to priority order) — the
/// ranking is an enhancement, never a hard dependency.
pub async fn kanban_ranked(
    state: &AppState,
    p: KanbanRankedParams,
) -> Result<KanbanRankedResult, ErrorObjectOwned> {
    let limit = p.limit.unwrap_or(DEFAULT_LIMIT);
    // Pull the whole roadmap board (priority-ordered by list_by_project), then
    // filter to runnable here so we rank over the real candidate set.
    let board = kanban(state, KanbanParams::new(p.project, 1000, None, None)).await?;
    let runnable: Vec<RankableCard> = board
        .items
        .into_iter()
        .filter(|i| is_runnable(&i.status))
        .map(|i| RankableCard {
            key: i.key,
            title: i.title,
            status: i.status,
        })
        .collect();

    // Empty prompt ⇒ priority order (no brain round-trip). Non-empty ⇒ rank by
    // brain.think recall; on brain error, fall back to priority order.
    let hit_ids: Vec<String> = if p.prompt.trim().is_empty() {
        Vec::new()
    } else {
        kavach_surreal::search_corpus(&state.db, &p.prompt, limit.saturating_mul(4))
            .await
            .map(|hits| hits.into_iter().map(|h| h.id).collect())
            .unwrap_or_default()
    };

    Ok(KanbanRankedResult {
        cards: rank_cards_by_relevance(runnable, &hit_ids, limit),
    })
}

#[cfg(test)]
#[path = "kanban_rank_test.rs"]
mod kanban_rank_test;

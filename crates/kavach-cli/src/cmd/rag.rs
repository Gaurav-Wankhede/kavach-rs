// TIME: O(1) per upsert (UNIQUE index `idx_rag_tree_source` + indexed entity lookup) | SPACE: O(n) trees + O(skill_count + edge_count)
// YEAR: 2026 | SEARCHED: 2026-05

mod build;
mod enrich;
mod graph;
mod query;
mod util;

use crate::cli::RagAction;

// Re-export handlers (called via run dispatcher only)
use build::{handle_build, handle_list, handle_refresh_if_stale};
use enrich::{handle_enrich, handle_enrich_skills};
use query::{handle_apply, handle_pending, handle_query};

/// `kavach rag <action>` — dispatch to the matching handler.
pub(super) fn run(action: RagAction) -> i32 {
    match action {
        RagAction::Build {
            source,
            label,
            persist,
        } => handle_build(&source, &label, persist),
        RagAction::Query {
            tree,
            file,
            text,
            intent,
            top_k,
        } => handle_query(&tree, &file, &text, &intent, top_k),
        RagAction::Pending { tree } => handle_pending(&tree),
        RagAction::Apply { tree, responses } => handle_apply(&tree, &responses),
        RagAction::List => handle_list(),
        RagAction::EnrichSkills { source, label } => handle_enrich_skills(&source, &label),
        RagAction::Enrich { source, label } => handle_enrich(&source, &label),
        RagAction::RefreshIfStale { source, label } => handle_refresh_if_stale(&source, &label),
    }
}

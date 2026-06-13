//! Stage 4 `[MISTAKE_GUARD]` advisory: cosine-retrieve the past mistakes most
//! relevant to THIS edit and reinject them as Reflexion negatives at the point
//! of action (loop-eng F2). The kNN runs daemon-side via `mistake.nearest`; this
//! leaf only embeds the query text (the content being written) into the request
//! and renders the do-instead rules. Best-effort: the daemon being down, an empty
//! graph, or a sub-floor match all degrade to `None` (advisory, never a block).
//
// ALGO: no local search — the cosine k-NN over the HNSW-indexed anti_pattern set
//   runs daemon-side (kavach-surreal nearest_anti_patterns); this leaf only
//   renders the <=k hits it returns. TIME: O(k) string build. SPACE: O(k).
//   YEAR: 2026.
use std::fmt::Write as _;

use crate::gates::pre_write_context::WriteContext;
use kavach_rpc::methods::mistake::{NearestParams, NearestResult};

/// How many relevant mistakes to surface — kept small so the advisory stays a
/// point-of-action nudge, not a wall of history.
const GUARD_K: usize = 3;
/// Below this query length the content is too thin to embed meaningfully (an
/// empty write, a one-line touch), so retrieval would only add noise.
const MIN_QUERY_LEN: usize = 32;

/// Build the `[MISTAKE_GUARD]` advisory for the edit described by `ctx`, or
/// `None` when there is nothing relevant to surface.
///
/// Query text is the effective body of the write (falling back to the raw
/// content) so the cosine match is against what is actually being authored.
/// Returns `None` on a too-short query, an unreachable daemon, or zero hits —
/// the gate must fail open.
pub(super) fn advisory(ctx: &WriteContext<'_>) -> Option<String> {
    let query = if ctx.effective_content.trim().is_empty() {
        ctx.content
    } else {
        ctx.effective_content.as_str()
    };
    if query.trim().len() < MIN_QUERY_LEN {
        return None;
    }

    let params = NearestParams::new(query.to_owned(), Some(GUARD_K), None);
    let result: NearestResult = kavach_rpc::client::call("mistake.nearest", Some(params)).ok()?;
    if result.hits.is_empty() {
        return None;
    }

    let mut out = String::from(
        "[MISTAKE_GUARD]\nRelevance-matched past mistakes (Reflexion negatives) for this edit \
         — do NOT repeat them; apply the do-instead rule before writing:\n",
    );
    for hit in &result.hits {
        // Writing to a String via fmt::Write is infallible; .ok() discards the
        // structurally-impossible fmt::Error explicitly (no band-aid let _).
        writeln!(
            out,
            "- {} (sim {:.2}): {}",
            hit.gate, hit.score, hit.correct_action
        )
        .ok();
    }
    out.push_str(
        "advisory: these are point-of-action lessons retrieved by cosine similarity to the \
         content being written — the closer the match, the more likely this edit is about to \
         re-make the same mistake.",
    );
    Some(out)
}

#[cfg(test)]
#[path = "mistake_guard_test.rs"]
mod mistake_guard_test;

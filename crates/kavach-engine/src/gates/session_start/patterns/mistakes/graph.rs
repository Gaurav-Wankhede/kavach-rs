//! Graph-backed `anti_pattern` reinjection — the read side of the autonomous
//! mistake loop. Calls `mistake.top` (the daemon's embedding-clustered
//! `anti_patterns`) so `SessionStart` reinforces the agent's OWN recurring
//! failures with the do-instead fix, pre-action. This is the half that was dark:
//! the default capture path writes graph `anti_patterns`, but reinjection used to
//! read only the legacy `pattern` `memory_entries`. On any RPC error the caller
//! falls back to that legacy ledger — boot must never block on memory injection.

//   result (ranking lives in kavach_surreal::graph_top_anti_patterns; see its
//   ALGO note). Here: one bounded loop building the reinjection string.
//   TIME: O(N), N ≤ REINJECT_TOP_N. SPACE: O(N). YEAR: 2026.
use std::fmt::Write as _;

use kavach_rpc::methods::mistake_top::{TopParams, TopResult};

/// How many top anti-patterns to reinject (matches the legacy path's cap).
const REINJECT_TOP_N: u32 = 5;

/// Top-N graph anti-patterns formatted for `SessionStart` reinjection, or `None`
/// when the daemon is unreachable or the graph holds no `anti_patterns` (the
/// caller then tries the legacy `pattern`-category ledger).
pub(super) fn anti_pattern_context() -> Option<String> {
    let res = kavach_rpc::client::call::<_, TopResult>(
        "mistake.top",
        Some(TopParams::new(Some(REINJECT_TOP_N))),
    )
    .ok()?;
    if res.patterns.is_empty() {
        return None;
    }
    let mut ctx = String::from(
        "\n[MISTAKE_LEDGER]\nstatus: anti-pattern reinforcement (graph, recurrence-ranked)\n",
    );
    for p in &res.patterns {
        writeln!(
            ctx,
            "- [hits={}] BANNED [{}] — INSTEAD: {}",
            p.hit_count, p.gate, p.correct_action
        )
        .ok();
    }
    ctx.push_str("rule: do NOT repeat any BANNED behavior above; apply the INSTEAD: fix pre-action.\n");
    Some(ctx)
}

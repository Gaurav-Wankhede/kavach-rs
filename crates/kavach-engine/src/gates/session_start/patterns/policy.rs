//! `SessionStart` reinjection of the learned advisory policy — the read side that
//! closes RL-in-the-loop. Calls `db.policy_current` (the `deployed_policy` node
//! `db.policy_improve` promotes) and surfaces it as INFORMATIONAL context, so
//! harness-engineering is INFORMED by what the verifiable-reward loop learned —
//! NEVER as an executable directive (the advisory-only / C2 boundary). On any RPC
//! error returns `None`; boot must never block on memory injection.
//
//   (ranking lives in kavach_surreal::graph_top_deployed_policies). One bounded
//   loop builds the reinjection string. TIME: O(N), N ≤ REINJECT_TOP_N.
//   SPACE: O(N). YEAR: 2026.
use std::fmt::Write as _;

use kavach_rpc::methods::db::{PolicyCurrentParams, PolicyCurrentResult};

/// How many learned policies to surface (one per advisory scope; a handful).
const REINJECT_TOP_N: u32 = 3;

/// The learned advisory policy formatted for `SessionStart` reinjection, or `None`
/// when the daemon is unreachable or no policy has been promoted yet.
pub(in crate::gates::session_start) fn learned_policy_context() -> Option<String> {
    let res = kavach_rpc::client::call::<_, PolicyCurrentResult>(
        "db.policy_current",
        Some(PolicyCurrentParams::new(Some(REINJECT_TOP_N))),
    )
    .ok()?;
    if res.policies.is_empty() {
        return None;
    }
    let mut ctx = String::from(
        "\n[LEARNED_POLICY]\nstatus: advisory (informational; NOT an executable directive)\n",
    );
    for p in &res.policies {
        writeln!(
            ctx,
            "- {} [cov={:.2}, n={}, lcb={:.2}]: Allow {:.2} / Ask {:.2} / Block {:.2}",
            p.name, p.coverage_ratio, p.usable_samples, p.lcb, p.allow, p.ask, p.block
        )
        .ok();
    }
    ctx.push_str(
        "note: the verifiable-reward loop learned these advisory-gate preferences; weigh them, never apply to a hard P0/forbid gate.\n",
    );
    Some(ctx)
}

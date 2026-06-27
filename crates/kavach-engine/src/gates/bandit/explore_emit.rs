//! Epsilon-greedy live wiring for the bandit emit path (harness-RL P7).
//!
//! The emit seam logged a CONSTANT propensity `1.0` (every gate decision was the
//! deterministic argmax), so the off-policy estimators in `kavach-ope`
//! (IPS/SNIPS/DR) had no overlap to evaluate a candidate policy — ESS collapsed
//! at propensity 1.0 and a learned controller could never be scored. P7 closes
//! that: when armed, the emit path draws an exploration sample and, with
//! probability `epsilon`, logs a NON-argmax advisory action together with its
//! TRUE propensity `< 1.0`. The pure selector + propensity arithmetic live in
//! `kavach_ope::explore`; this module is only the live wiring — the env arming,
//! the seed, and the advisory candidate set.
//!
//! SAFETY (design C2, authorized by kavach `decision.harness-rl.c1-exploration-
//! authorized`): exploration is ADVISORY-SCOPE ONLY. The candidate set passed to
//! the selector is `{Allow, Ask}` — `Block` (the hard-stop / P0-equivalent action)
//! is NEVER a candidate, and `epsilon_greedy_select` provably never returns an
//! action outside its candidate set, so exploration can never turn a permit into a
//! block. A greedy `Block` (a hard-block decision) falls back to `(Block, 1.0)`
//! because it is absent from the advisory set — the selector's own misuse guard.
//!
//! Disarmed (the default), [`explore_action`] returns `(greedy, 1.0)` — the exact
//! pre-P7 deterministic behavior.
use kavach_ope::explore::{epsilon_greedy_select, next_draw};
use kavach_patterns::bandit_log::GateAction;
/// Env flag that arms live exploration. Absent/empty/`"0"`/`"false"` ⇒ disarmed.
const EXPLORE_FLAG: &str = "KAVACH_RL_EXPLORE";
/// Env override for the exploration rate; falls back to [`DEFAULT_EPSILON`].
const EPSILON_FLAG: &str = "KAVACH_RL_EPSILON";
/// Default exploration rate when armed without an explicit `KAVACH_RL_EPSILON`.
/// A small band: most turns still emit the greedy action, but enough mass leaks
/// onto the abstention action to give the estimators non-degenerate overlap.
const DEFAULT_EPSILON: f32 = 0.1;
/// The advisory action set the emit path may explore over. `Block` is
/// deliberately absent (the C2 hard-block bar) — exploration can only ever move
/// between permitting and the safe abstention `Ask`, never synthesize a stop.
const ADVISORY_CANDIDATES: [GateAction; 2] = [GateAction::Allow, GateAction::Ask];
/// Resolve the action + TRUE propensity to log for a decision whose deterministic
/// (argmax) choice is `greedy`.
///
/// Disarmed ⇒ `(greedy, 1.0)`. Armed ⇒ epsilon-greedy over the advisory set,
/// seeded deterministically from `(timestamp_ms, session_id)` so the emit stays
/// reproducible from its logged row (no hidden RNG state). A `greedy` outside the
/// advisory set (e.g. `Block`) falls back to `(greedy, 1.0)` via the selector's
/// misuse guard — a hard decision is logged honestly as deterministic.
#[must_use]
pub(crate) fn explore_action(
    greedy: GateAction,
    session_id: &str,
    timestamp_ms: i64,
) -> (GateAction, f32) {
    let Some(epsilon) = armed_epsilon() else {
        return (greedy, 1.0);
    };
    let mut state = seed(session_id, timestamp_ms);
    let draw = next_draw(&mut state);
    epsilon_greedy_select(greedy, &ADVISORY_CANDIDATES, epsilon, draw)
}
/// A uniform draw in `[0,1)` for the P8 held-out sampler, seeded independently
/// from the exploration draw so the two samplings never correlate.
///
/// Same dep-free `xorshift64*` as [`explore_action`], but the seed is salted with
/// a distinct constant before mixing — otherwise a turn that explores would also
/// be the turn that gets held out (or never), coupling two decisions that must be
/// statistically independent.
#[must_use]
pub(crate) fn held_out_roll(session_id: &str, timestamp_ms: i64) -> f32 {
    // Salt the seed so this draw is decorrelated from the exploration draw.
    let mut state = seed(session_id, timestamp_ms) ^ 0x9E37_79B9_7F4A_7C15;
    state |= 1; // never the xorshift fixed point after the salt XOR
    next_draw(&mut state)
}
/// The exploration rate IFF exploration is armed this process, else `None`.
///
/// Armed = `KAVACH_RL_EXPLORE` truthy. The rate is `KAVACH_RL_EPSILON` when it
/// parses to a finite value, otherwise [`DEFAULT_EPSILON`]; the selector clamps it
/// to `[0,1]`, so a hostile out-of-range value cannot widen exploration past 1.0.
fn armed_epsilon() -> Option<f32> {
    if !std::env::var(EXPLORE_FLAG).is_ok_and(|v| is_truthy(&v)) {
        return None;
    }
    let eps = std::env::var(EPSILON_FLAG)
        .ok()
        .and_then(|v| v.trim().parse::<f32>().ok())
        .filter(|e| e.is_finite())
        .unwrap_or(DEFAULT_EPSILON);
    Some(eps)
}
/// Only an explicit truthy arms exploration (mirrors the canary flag semantics).
fn is_truthy(v: &str) -> bool {
    matches!(
        v.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}
/// A non-zero `xorshift64*` seed from the decision's identity.
///
/// `timestamp_ms` mixed with a cheap FNV-1a hash of `session_id` so two sessions
/// deciding in the same millisecond explore independently. The low bit is forced
/// set so the seed is never zero (the xorshift fixed point). This is exploration
/// sampling, not cryptography — distribution, not unpredictability, is what the
/// propensity bookkeeping needs.
fn seed(session_id: &str, timestamp_ms: i64) -> u64 {
    // Reinterpret the bits (cast_unsigned, lossless): only distribution matters
    // for an exploration seed, and the low-bit force-set rescues the all-zero case.
    let ts = timestamp_ms.cast_unsigned();
    (ts ^ fnv1a(session_id)) | 1
}
/// FNV-1a 64-bit over the session id — a dep-free, well-mixing string hash for the
/// exploration seed (not a security or content-addressing hash).
fn fnv1a(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}
#[cfg(test)]
#[path = "explore_emit_test.rs"]
#[path = "explore_emit_test.rs"]
mod tests;

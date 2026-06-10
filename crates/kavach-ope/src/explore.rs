//! Epsilon-greedy exploration over an advisory action set (harness-RL P7 core).
//!
//! Authorized 2026-06-10 (kavach `decision.harness-rl.c1-exploration-authorized`):
//! the live gate may, with probability `epsilon`, emit a NON-argmax ADVISORY
//! action and log the TRUE propensity, giving the off-policy estimators
//! (IPS/SNIPS/DR in this crate) non-degenerate overlap so a candidate policy can
//! actually be evaluated instead of collapsing ESS at propensity 1.0.
//!
//! SAFETY (design C2 — preserved here): exploration is advisory-scope ONLY. This
//! function never invents an action outside `candidates`, so when the caller
//! passes an `AdvisoryCandidates` set (P0/forbid barred structurally) a forbidden
//! action can never be selected. A P0 gate never reaches this path at all.
//!
//! The selection is a PURE function of `draw` (the caller supplies the uniform
//! random sample, mirroring how the emit path takes the timestamp), so the
//! propensity arithmetic is tested without an RNG. [`next_draw`] is the dep-free
//! xorshift the live call site uses to produce that draw (`rand` is not a
//! workspace dependency; this is exploration sampling, not cryptography).

/// Select an advisory action under epsilon-greedy with its true propensity.
///
/// The propensity is the probability the policy assigns the returned action —
/// exactly what the off-policy estimators divide by. Generic over the action
/// type; the live gate instantiates it with
/// `kavach_patterns::bandit_log::GateAction`. `greedy` is the controller's argmax
/// choice and MUST be one of `candidates` (the full advisory set it chose from,
/// assumed distinct). `epsilon` is clamped to `[0,1]`; `draw` is uniform in
/// `[0,1)`.
///
/// Propensities over the candidate set sum to 1: the greedy action carries
/// `1 - epsilon + epsilon/|A|`, every other candidate `epsilon/|A|`. Degenerate
/// inputs — fewer than two candidates, `epsilon == 0`, or a `greedy` not in the
/// set — fall back to `(greedy, 1.0)`, i.e. the exact pre-P7 deterministic
/// behavior, which is the fail-safe default when exploration is off.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "|A| is a tiny distinct candidate count (exact in f32); the index is \
              an intentional floor of a value provably in [0, |A|)"
)]
pub fn epsilon_greedy_select<A: Copy + PartialEq>(
    greedy: A,
    candidates: &[A],
    epsilon: f32,
    draw: f32,
) -> (A, f32) {
    let eps = epsilon.clamp(0.0, 1.0);
    let n = candidates.len();
    if n <= 1 || eps == 0.0 || !candidates.contains(&greedy) {
        return (greedy, 1.0);
    }
    let a = n as f32;
    let greedy_p = 1.0 - eps + eps / a;
    if draw < eps {
        // Explore: map `draw` uniformly across the FULL set (so each action,
        // greedy included, carries epsilon/|A| of its mass — landing back on
        // greedy is correct and simply yields no behavior change this turn).
        let idx = (((draw / eps) * a) as usize).min(n.saturating_sub(1));
        let chosen = candidates.get(idx).copied().unwrap_or(greedy);
        let p = if chosen == greedy { greedy_p } else { eps / a };
        (chosen, p)
    } else {
        (greedy, greedy_p)
    }
}

/// A dep-free uniform draw in `[0,1)` from a `xorshift64*` state, advancing it.
///
/// `state` must be non-zero (zero is the xorshift fixed point); seed the live
/// call site from `now_ns XOR session_hash` with the low bit forced set. A
/// well-distributed `xorshift64*` is sufficient for exploration sampling — the
/// propensity (what the estimators need) comes from `epsilon`, not the RNG.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "the high 24 bits are < 2^24, so the widening to f32 is exact"
)]
pub fn next_draw(state: &mut u64) -> f32 {
    let mut x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    let v = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
    // Top 24 bits → [0,1): a 24-bit mantissa is f32's exact integer range.
    (v >> 40) as f32 / 16_777_216.0
}

#[cfg(test)]
#[path = "explore_test.rs"]
mod tests;

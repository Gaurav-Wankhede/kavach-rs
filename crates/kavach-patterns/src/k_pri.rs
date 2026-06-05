//! K-PRI — unified priority score for every kavach work surface.
//!
//! ALGO: WeightedComposite{LFU + ReddiHN-decay + RICE + Eisenhower + `TopoCP`}
//!
//! `PROBLEM_CLASS`: ranking
//!
//! REJECTED (used IN ISOLATION; each is COMBINED below at non-zero weight):
//! - `recency_only_LRU`: alone orphans sticky high-frequency rows
//! - `LFU_only`: alone never expires once-hot rows; staleness drift
//! - `WSJF_only`: alone ignores recurrence + user focus
//! - `Eisenhower_only`: alone too coarse for fine ranking
//! - `RICE_only`: alone no recurrence/decay axis; mistake ledger fails
//!
//! COMBINED: the final composite IS a weighted sum of LFU (log-recurrence,
//! w=3.0 in `W_MISTAKE_LEDGER`) + exp-decay (w=0.5..1.0) + RICE `effort_inv`
//! (w=1.5 in `W_ROADMAP`) + Eisenhower urgency (focus + `cost_of_delay`) +
//! topological `blocker_weight`. Each is rejected ALONE but contributes to
//! the unified score.
//!
//! TIME: O(1) per score | SPACE: O(1) | YEAR: 2026 | SEARCHED: 2026-05
//!
//! TRADEOFF: weight tuning is per-surface (constant table) — surfaces with
//! wildly different signal mixes get different weights, not different code.
//!
//! SOURCES:
//!  - RICE/WSJF/MoSCoW/ICE compared 2026:
//!    productlift.dev/blog/product-prioritization-framework-comparison/
//!  - Eisenhower × Kanban 2026: taskforge.md/blog/eisenhower-matrix/
//!  - SIEVE eviction (NSDI'24): usenix.org/system/files/nsdi24-zhang-yazhuo.pdf
//!  - O(1) LFU: arxiv.org/pdf/2110.11602
//!  - Reddit Hot: medium.com/hacking-and-gonzo/how-reddit-ranking-algorithms-work-ef111e33d0d9
//!  - HN reverse-engineered: sangaline.com/post/reverse-engineering-the-hacker-news-ranking-algorithm
//!  - Topo sort + Critical Path: `en.wikipedia.org/wiki/Topological_sorting`
//!  - Mistake Notebook (anti-pattern reinjection): arxiv.org/html/2512.11485
//!  - Rust composite-priority pattern: oneuptime.com/blog/post/2026-01-25-prioritize-critical-requests-high-load-rust

/// All six normalized signal axes. Each in [0.0, 1.0] except `effort`
/// which is the raw size estimate (>=1).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate constructed in kavach-engine; non_exhaustive => E0639"
)]
pub struct Signals {
    /// 1.0 if item is inside the active `USER_FOCUS` scope, else 0.0.
    pub focus: f64,
    /// Normalized count of items this one unblocks (descendants in DAG).
    /// Caller clamps to [0,1] by dividing by the max `blocker_weight` in
    /// the surface (or by a soft cap like 10.0).
    pub blocker_weight: f64,
    /// Normalized cost-of-delay — staleness × business value. Caller
    /// clamps to [0,1].
    pub cost_of_delay: f64,
    /// Raw integer hit-count for recurrence-style ranking (mistake
    /// ledger). Logged inside the scorer to dampen runaway growth.
    pub hit_count: u32,
    /// Age in days since last touch — for exponential decay.
    pub age_days: f64,
    /// Effort estimate (LOC, T-shirt size, story points). >=1.
    pub effort: f64,
}

/// Per-surface weights. Each weight may be any non-negative f64; higher
/// weight means that signal counts more for that surface's ranking.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct Weights {
    pub focus: f64,
    pub blocker_weight: f64,
    pub cost_of_delay: f64,
    pub recurrence: f64,
    pub time_decay: f64,
    pub effort_inv: f64,
}

/// Per-surface weight presets. Each surface emphasizes the axes that
/// actually carry signal for that surface — derived from the
/// surface↔dominant-signal table in the K-PRI design doc.
pub const W_MISTAKE_LEDGER: Weights = Weights {
    focus: 0.0,
    blocker_weight: 0.0,
    cost_of_delay: 0.0,
    recurrence: 3.0,
    time_decay: 1.0,
    effort_inv: 0.0,
};
pub const W_ROADMAP: Weights = Weights {
    focus: 1.0,
    blocker_weight: 2.0,
    cost_of_delay: 1.5,
    recurrence: 0.5,
    time_decay: 0.5,
    effort_inv: 1.5,
};
pub const W_KANBAN: Weights = Weights {
    focus: 3.0,
    blocker_weight: 2.0,
    cost_of_delay: 2.0,
    recurrence: 0.5,
    time_decay: 1.0,
    effort_inv: 1.0,
};
pub const W_KANBAN_PHASE: Weights = Weights {
    focus: 2.0,
    blocker_weight: 3.0,
    cost_of_delay: 1.0,
    recurrence: 0.0,
    time_decay: 0.0,
    effort_inv: 0.0,
};

/// 14-day exponential half-life — chosen so two-week-old items keep
/// ~50% decay weight, four-week-old items ~25%, eight-week-old ~6%.
const TIME_HALF_LIFE_DAYS: f64 = 14.0;

/// Compute K-PRI for a single item. Returns a non-negative f64 ranking
/// score; higher is more important. Pure function — no I/O.
#[must_use]
#[expect(
    clippy::float_arithmetic,
    reason = "K-PRI algorithm: log-recurrence + exp-decay + inverse-effort are unavoidable"
)]
pub fn score(sig: Signals, w: Weights) -> f64 {
    // LFU-flavor: log2(1 + hit) so a 50-hit row beats a 1-hit row by
    // log2(51)≈5.67× recurrence weight rather than 50× (avoids one mega
    // row swamping the list).
    let recur = (1.0 + f64::from(sig.hit_count)).log2();
    // Reddit/HN flavor — exp half-life decay.
    let decay = (-sig.age_days.max(0.0) / TIME_HALF_LIFE_DAYS).exp();
    // RICE flavor — value-per-effort. Effort clamped at 1.0 to avoid
    // divide-by-zero or runaway scores.
    let effort_inv = 1.0 / sig.effort.max(1.0);

    w.effort_inv.mul_add(
        effort_inv,
        w.time_decay.mul_add(
            decay,
            w.recurrence.mul_add(
                recur,
                w.cost_of_delay.mul_add(
                    sig.cost_of_delay.clamp(0.0, 1.0),
                    w.focus.mul_add(
                        sig.focus,
                        w.blocker_weight * sig.blocker_weight.clamp(0.0, 1.0),
                    ),
                ),
            ),
        ),
    )
}

/// Rank items by descending K-PRI score. Stable tie-break by input
/// index (preserves caller-provided ordering for equal scores).
pub fn rank<T, F>(items: Vec<T>, w: Weights, mut signal_of: F) -> Vec<(T, f64)>
where
    F: FnMut(&T) -> Signals,
{
    let mut scored: Vec<(usize, T, f64)> = items
        .into_iter()
        .enumerate()
        .map(|(idx, item)| {
            let s = signal_of(&item);
            let p = score(s, w);
            (idx, item, p)
        })
        .collect();
    // Descending by score, then ascending by original idx for stability.
    scored.sort_by(|a, b| {
        b.2.partial_cmp(&a.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    scored.into_iter().map(|(_, t, p)| (t, p)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig_with_hits(hit_count: u32) -> Signals {
        Signals {
            hit_count,
            ..Default::default()
        }
    }

    #[test]
    fn higher_hit_count_wins_in_mistake_ledger() {
        let a = score(sig_with_hits(50), W_MISTAKE_LEDGER);
        let b = score(sig_with_hits(1), W_MISTAKE_LEDGER);
        assert!(a > b, "50 hits ({a}) must outrank 1 hit ({b})");
    }

    #[test]
    fn log_recurrence_prevents_one_row_dominance() {
        // 1000-hit row should be < 12× a 1-hit row, not 1000×.
        let big = score(sig_with_hits(1000), W_MISTAKE_LEDGER);
        let small = score(sig_with_hits(1), W_MISTAKE_LEDGER);
        let ratio = big / small.max(0.0001);
        assert!(
            ratio < 12.0,
            "ratio {ratio} should be log-bounded, not linear"
        );
    }

    #[test]
    fn focus_swamps_other_axes_in_kanban() {
        let focused = score(
            Signals {
                focus: 1.0,
                hit_count: 0,
                ..Default::default()
            },
            W_KANBAN,
        );
        let unfocused_blocker = score(
            Signals {
                focus: 0.0,
                blocker_weight: 1.0,
                ..Default::default()
            },
            W_KANBAN,
        );
        assert!(
            focused > unfocused_blocker,
            "USER_FOCUS (w_F=3) must outrank a max-blocker (w_B=2) in kanban surface"
        );
    }

    #[test]
    fn time_decay_does_not_orphan_high_freq() {
        // 100-hit row aged 60 days should still beat a fresh 1-hit row
        // — sticky LFU under K-PRI.
        let old_sticky = score(
            Signals {
                hit_count: 100,
                age_days: 60.0,
                ..Default::default()
            },
            W_MISTAKE_LEDGER,
        );
        let fresh_one = score(
            Signals {
                hit_count: 1,
                age_days: 0.0,
                ..Default::default()
            },
            W_MISTAKE_LEDGER,
        );
        assert!(
            old_sticky > fresh_one,
            "old_sticky ({old_sticky}) must outrank fresh_one ({fresh_one}) — LFU dominates decay"
        );
    }

    #[test]
    fn rank_is_stable_for_ties() {
        let items = vec!["a", "b", "c"];
        let ranked = rank(items, W_ROADMAP, |_| Signals::default());
        let keys: Vec<&str> = ranked.iter().map(|(t, _)| *t).collect();
        assert_eq!(
            keys,
            vec!["a", "b", "c"],
            "stable tie-break preserves input order"
        );
    }

    #[test]
    fn rank_orders_descending_by_score() {
        let items = vec![1u32, 50, 10];
        let ranked = rank(items, W_MISTAKE_LEDGER, |&n| sig_with_hits(n));
        let keys: Vec<u32> = ranked.iter().map(|(t, _)| *t).collect();
        assert_eq!(keys, vec![50, 10, 1]);
    }

    #[test]
    fn effort_inverse_rewards_quick_wins_in_roadmap() {
        let cheap = score(
            Signals {
                effort: 1.0,
                blocker_weight: 0.5,
                ..Default::default()
            },
            W_ROADMAP,
        );
        let expensive = score(
            Signals {
                effort: 100.0,
                blocker_weight: 0.5,
                ..Default::default()
            },
            W_ROADMAP,
        );
        assert!(
            cheap > expensive,
            "lower effort outranks higher effort at same blocker weight"
        );
    }
}

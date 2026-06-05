// split: Un-gameable reward scorer over an eval_replay trajectory (INV-1 of the
// GEPA harness self-optimization plan). WHY this weighting + the reward-hack RCA:
// kavach-db decision.harness-reward-ungameable / roadmap.unit.harness-self-optimization.

use crate::eval_replay::{EventKind, ReplaySeverity, TrajectoryEvent, replay_event};
use regex::Regex;
use std::sync::LazyLock;

// Witness weights (INV-1): real verification dominates; a bare test is worth
// nothing. VACUOUS_TEST = 0 is the reward-hack guard (AC-5) — an always-pass test
// earns only the FILE_LANDED floor, never out-scoring a real build.
const REAL_BUILD_OK: i64 = 10; // real `cargo check`/`build` exit-0
const REAL_TEST_RUN: i64 = 4; // real `cargo test`/`nextest` or substantive test
const FILE_LANDED: i64 = 1; // a write that landed (cheap alone)
const VACUOUS_TEST: i64 = 0; // always-pass / no-op test — DELIBERATELY zero
const GATE_BLOCK_PENALTY: i64 = -6; // per P0-Block the events triggered

/// Real verification commands: a `cargo check`/`build`/`test`/`nextest` in command
/// position (start, after `&&`/`;`/`|`, or after a `VAR=` prefix). Mirrors the
/// command-position discipline of the `test_tracker` so a quoted mention is not a hit.
static REAL_VERIFY: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"(?:^|&&|;|\|)\s*(?:[A-Z_][A-Z0-9_]*=\S+\s+)*cargo\s+(check|build|test|nextest)\b")
        .ok()
});

/// An always-pass / vacuous test body — the reward-hack shape AC-5 must neutralize.
/// `assert!(true)`, `assert_eq!(1, 1)`, or an empty `#[test] fn x() {}`.
// NOTE: the `regex` crate is finite-automata based — NO backreferences. The
// "identical sides" check (`assert_eq!(x, x)`) is enumerated for the literals that
// actually appear in no-op tests rather than via a `\1` backref (unsupported).
static VACUOUS_RE: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(
        r"assert!\s*\(\s*true\s*\)|assert_eq!\s*\(\s*(?:true\s*,\s*true|1\s*,\s*1|0\s*,\s*0)\s*\)|#\[test\]\s*(?:async\s+)?fn\s+\w+\s*\(\s*\)\s*\{\s*\}",
    )
    .ok()
});

/// A test that exercises real code: has a `#[test]`/`#[tokio::test]` attribute.
static SUBSTANTIVE_RE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"#\[test\]|#\[tokio::test\]").ok());

fn matches(re: &LazyLock<Option<Regex>>, hay: &str) -> bool {
    re.as_ref().is_some_and(|r| r.is_match(hay))
}

/// The reward contribution of a single event.
fn score_event(event: &TrajectoryEvent) -> i64 {
    // Every event still flows through the live gate set: a Block is a penalty.
    let gate_penalty: i64 = i64::try_from(
        replay_event(event)
            .iter()
            .filter(|o| o.severity == ReplaySeverity::Block)
            .count(),
    )
    .map_or(0, |n| n.saturating_mul(GATE_BLOCK_PENALTY));

    let signal = match &event.event_kind {
        EventKind::Bash { command } => {
            if matches(&REAL_VERIFY, command) {
                // `cargo test`/`nextest` is a test run; check/build is the strong signal.
                if command.contains("test") || command.contains("nextest") {
                    REAL_TEST_RUN
                } else {
                    REAL_BUILD_OK
                }
            } else {
                0
            }
        }
        EventKind::Write { content, .. } => {
            let mut s = FILE_LANDED;
            // A test was written. Reward it ONLY if it is substantive — a vacuous
            // always-pass test earns VACUOUS_TEST (zero). This is AC-5.
            if matches(&SUBSTANTIVE_RE, content) {
                s = s.saturating_add(if matches(&VACUOUS_RE, content) {
                    VACUOUS_TEST
                } else {
                    REAL_TEST_RUN
                });
            }
            s
        }
        EventKind::Tool { .. } | EventKind::Stop { .. } => 0,
    };

    signal.saturating_add(gate_penalty)
}

/// The deterministic, un-gameable reward of a whole trajectory (AC-1).
///
/// Read-only over `events` (INV-2): it never mutates state and never participates
/// in the session it scores. Same input -> same output.
#[must_use]
pub fn score_trajectory(events: &[TrajectoryEvent]) -> i64 {
    events
        .iter()
        .map(score_event)
        .fold(0_i64, i64::saturating_add)
}

#[cfg(test)]
#[path = "reward_test.rs"]
mod tests;

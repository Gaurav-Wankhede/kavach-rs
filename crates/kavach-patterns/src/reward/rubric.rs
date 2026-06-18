//! Project-adaptive reward rubric — a weighted signal vector.
//!
//! Owner directive 2026-06-17: "expand the RLAIF — each project has different
//! patterns and tech stacks". Each [`SignalRule`] matches an event class via a
//! regex and contributes a signed weight. The default rubric reproduces the
//! Rust/cargo weights verbatim; a project supplies extra/override rules via the
//! `gate.reward_rubric` DB row (DATA, no rebuild — mirrors `gate.dispatch_directive`).

use regex::Regex;

/// Which event class a [`SignalRule`] applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EventClass {
    /// A `Bash` command (verify runs: build/test/lint/migrate).
    Bash,
    /// A `Write` file body (test authored, file landed).
    Write,
    /// The turn's final `Stop` message (deferral-handoff detection).
    Stop,
}

/// One weighted signal in a rubric.
///
/// `pattern` (matched against the event's text) contributes `weight` (signed:
/// credit positive, debit negative) when it fires on an event of class
/// `applies_to`. `id` is a stable tag for the ledger line item.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SignalRule {
    /// Stable ledger tag, e.g. `build` / `test` / `gate_block` / `deferral_handoff`.
    pub id: &'static str,
    /// Event class this rule scores.
    pub applies_to: EventClass,
    /// Regex matched against the event text (command / file body / stop message).
    pub pattern: Regex,
    /// Signed point contribution when the pattern fires.
    pub weight: i64,
}

/// A project's full reward rubric: the ordered signal rules + an optional
/// vacuous-guard that zeroes a `Write` credit when the body is a no-op test.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RewardRubric {
    /// All signal rules (credits + debits), evaluated in order; weights sum.
    pub rules: Vec<SignalRule>,
    /// When set and a `Write` body matches, its credit rules are neutralized
    /// (the reward-hack guard — an always-pass test earns only the file floor).
    pub vacuous_guard: Option<Regex>,
}

impl RewardRubric {
    /// Construct from already-compiled rules (the preset builders use this).
    #[must_use]
    pub const fn new(rules: Vec<SignalRule>, vacuous_guard: Option<Regex>) -> Self {
        Self { rules, vacuous_guard }
    }
}

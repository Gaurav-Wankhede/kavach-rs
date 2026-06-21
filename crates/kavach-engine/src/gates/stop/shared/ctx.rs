//! `StopCtx` — the per-invocation mutable context threaded through every
//! stop-gate guard's `check()`. Single responsibility: bundle the state so
//! every guard has the uniform signature `check(&mut StopCtx) -> ControlFlow`.

use kavach_types::HookInput;

/// Per-invocation mutable context threaded through every guard's `check()`.
/// Bundling these into one struct gives every guard a uniform signature
/// (`check(ctx: &mut StopCtx<'_>) -> ControlFlow<()>`) so the driver in
/// stop.rs is a flat ordered list with no per-guard argument plumbing.
pub(crate) struct StopCtx<'a> {
    /// The raw Stop hook input (immutable for the whole pipeline).
    pub(crate) input: &'a HookInput,
    /// Live session state; guards mutate the breaker, card pointer, etc.
    pub(crate) session: &'a mut kavach_session::SessionState,
    /// P1 semver advisory slot, read by the terminal clean-exit guard. The
    /// bounty scan that used to populate it was removed under the "kill
    /// blocking, keep auto-continue" policy, so it is currently always `None`;
    /// the field is retained as the clean-exit context slot.
    pub(crate) semver_advisory: Option<String>,
    /// Capture-finding advisory (U3): set when the final message settled a
    /// decision in prose that was NOT persisted to the DB this turn. Appended to
    /// the clean-exit STOP context as a NON-blocking nudge — never a HALT.
    pub(crate) capture_advisory: Option<String>,
    /// Loophole self-interrogation advisory: set when the turn claimed
    /// completion on a risk-bearing path WITHOUT a `Loopholes closed:` line.
    /// On a drained board the clean-exit terminal REFUSES the stop while this is
    /// present (parity with `[CYCLE_DEADLOCK]`), bounded by the `loophole_open`
    /// behavioral breaker; the omission is also recorded to the mistake ledger at
    /// the computation site so the learning loop sees it on every stop.
    pub(crate) loophole_advisory: Option<String>,
    /// Shallow-verdict advisory: set when the turn asserted a clean/wired/
    /// no-defect verdict with no `file:line` citation and no `[RCA]` block — the
    /// shallow-research signature. Appended to the clean-exit context as a
    /// NON-blocking nudge; recorded to the mistake ledger at the computation site
    /// so the learning loop sees it on every stop, not just clean exits.
    pub(crate) shallow_advisory: Option<String>,
    /// Continuation-menu advisory: set when the final assistant message ENDED
    /// THE TURN on a "continue or pause?" / "want me to proceed, or redirect?"
    /// permission question while the loop directive (the `[AUTO_CONTINUE]` verdict
    /// this very gate emits) already commands autonomous continuation. The model
    /// satisfied the loop in the gate's OUTPUT while its own final message asked
    /// the user for permission to do what the gate already ordered — the exact
    /// loop-stall the user reported. Appended to the clean-exit context as a
    /// NON-blocking nudge (loop-safe — the next turn either continues or, if the
    /// board is genuinely drained, STATES the clean stop without a question);
    /// recorded to the mistake ledger at the computation site so the learning loop
    /// sees it on every stop.
    pub(crate) continuation_advisory: Option<String>,
    /// Research-witness signal: set when `detect_claim_without_research` fired this
    /// turn — the final message asserted a current-knowledge fact (latest/version/
    /// API/pricing/supports) from memory with no source URL. On a drained board the
    /// clean-exit terminal REFUSES the stop while this is true (parity with the
    /// loophole / roadmap-todos refuse-stops), bounded by the `research_unsourced`
    /// behavioral breaker — giving internet-first (global CLAUDE.md) teeth instead
    /// of an advisory the model can coast past.
    pub(crate) research_unsourced: bool,
}

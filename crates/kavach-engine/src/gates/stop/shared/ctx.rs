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
}

//! `[AUTO_CONTINUE]` dispatch-envelope builder.
//!
//! The binary carries NO procedure prose. It emits STATE (card key, status,
//! claim-state, shape signals) + the project's DYNAMIC directive text fetched
//! from the kavach DB (`decision` row `gate.dispatch_directive`). The operator edits
//! that row to change gate behavior per-project — no rebuild. Absent directive →
//! a minimal generic fallback so a fresh project still loops.

/// Inputs to the dispatch-envelope builder (`§RUST_STRICT`: struct over many args).
pub(super) struct EnvelopeCtx<'a> {
    pub proj: &'a str,
    pub priority: &'a str,
    pub title: &'a str,
    pub loop_prefix: &'a str,
    pub reward_prefix: &'a str,
    pub claimed: bool,
    pub persisted_in_progress: bool,
    /// Project's dynamic directive text from the DB; `None` → minimal fallback.
    pub directive: Option<&'a str>,
}

/// Emit `{prefixes}[AUTO_CONTINUE] {state facts}\n{directive}`.
///
/// State facts are objective (key, status, claim-state, shape signals); the
/// directive is operator-authored DATA. No decompose/reconcile/RLAIF prose is
/// compiled in — the LLM reasons from the project's own directive + the card.
pub(super) fn dispatch_envelope(c: &EnvelopeCtx<'_>) -> String {
    let claim_state = match (c.claimed, c.persisted_in_progress) {
        (true, true) => "CLAIMED + in_progress (DB confirmed)",
        (true, false) => "claim issued, DB not yet in_progress (persist before narrating)",
        (false, _) => "already in_progress (resume)",
    };
    let needs_decomp = kavach_rpc::methods::roadmap::readiness::is_needs_decomposition(c.title);
    // Gate-shaped title (`GATE:`/owner-gate/CLASS-B): strip the gate words so the
    // agent sees the WORK, and prepend an ACT imperative that forbids "Holding"
    // and names the split/delete exits — kills the owner-gate dispatch loop.
    let gate_shaped = super::gate_strip::is_gate_shaped(c.title);
    let shown_title = if gate_shaped {
        super::gate_strip::strip_gate_words(c.title)
    } else {
        c.title.to_owned()
    };
    let shape = if gate_shaped {
        "  shape: GATE-STRIPPED (owner-gate words removed — this is runnable work)\n"
    } else if needs_decomp {
        "  shape: NEEDS-DECOMPOSITION (title declares not-one-card)\n"
    } else {
        ""
    };
    let directive = c.directive.unwrap_or(FALLBACK_DIRECTIVE);
    // For a gate card the act-imperative leads (never a hold); the project's
    // dispatch directive rides along after it.
    let directive_block = if gate_shaped {
        format!(
            "{}\n\n{directive}",
            super::gate_strip::act_imperative(&shown_title)
        )
    } else {
        directive.to_owned()
    };
    format!(
        "{lp}{rp}[AUTO_CONTINUE] Do NOT stop — work the dispatched card THIS turn.\n\
         STATE:\n\
         \x20 card: {key}\n\
         \x20 title: {shown_title}\n\
         \x20 claim: {claim_state}\n\
         {shape}\
         \x20 read: kavach db get --project {proj} --category roadmap --key {key} --full\n\n\
         DIRECTIVE (project-authored; the gate carries no fixed procedure):\n\
         {directive_block}",
        lp = c.loop_prefix,
        rp = c.reward_prefix,
        key = c.priority,
        proj = c.proj,
    )
}

/// Minimal fallback when the project has no `gate.dispatch_directive` row. Kept
/// deliberately generic + short — real guidance is the project's DB directive.
const FALLBACK_DIRECTIVE: &str = "No project directive set (decision row `gate.dispatch_directive` absent). \
     Read the card + this project's `.claude/rules/*` + decision rows, decide the \
     honest next action autonomously (build / decompose / dep-gate / verify / delete), \
     execute it this turn, and verify before claiming done. Set a directive row to \
     customize: kavach db write --category decision --key gate.dispatch_directive.";

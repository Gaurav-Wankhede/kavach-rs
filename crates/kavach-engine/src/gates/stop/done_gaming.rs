//! Guard (`P0Block`, non-surrenderable): REFUSE a stop that games completion —
//! decorated status-block narration ("✅ DONE", "vacuously complete",
//! "documentation pass", "await features") emitted while real work remains and
//! this turn produced NO code/DB mutation. Naming work done is NOT doing it
//! (`§4a_describe_is_not_done`). Closes the "done-by-redefinition" loophole the
//! advisory detectors only nudge.
//!
//! FIRES on the three-condition AND (the false-positive bound):
//!
//! 1. the final message carries gaming/narration vocabulary, AND
//! 2. the kavach DB census shows `runnable > 0` (real dispatchable work), AND
//! 3. NO real mutation happened this turn (a `.md`/doc write or a lone
//!    decision-row write does NOT count — that IS the gaming path).
//!
//! Any one condition false → does NOT fire. Census `None` (RPC outage) →
//! fail-OPEN (never wedge on an unobservable board). NEG-arm: narration that
//! ACCOMPANIES real proof (`git diff --stat` / `cargo check` / an `rg` `file:line`
//! / a `claim`/`status-update`) never fires. Escape: `KAVACH_DONE_GAMING_BYPASS=1`
//! for a genuine doc-only deliverable turn (logged via the block being skipped).

use core::ops::ControlFlow;
use std::path::Path;

use super::shared::StopCtx;

/// Done-by-redefinition + narration/sign-off vocabulary. Lower-cased substring
/// match. Kept deliberately specific — each phrase is a completion-claim the model
/// uses to END a turn without doing the work, NOT generic prose.
const GAMING_PHRASES: &[&str] = &[
    // done-by-redefinition
    "vacuously complete",
    "vacuous",
    "await features",
    "awaiting features",
    "shipped for live types",
    "documentation pass",
    "doc pass",
    "safe-by-construction",
    "nothing further is runnable",
    "must not run",
    // narration / sign-off
    "the new status block",
    "status block:",
    "that completes",
    "one-line summary",
    "single source of truth for the migration",
];

/// Operator-handback / surrender phrases — the abolished "push it to the owner"
/// pattern that re-leaked under infra-stress (the ENOSPC transcript: "Owner — run
/// `rm -rf …`", "owner-authorization anchor", spinning "Holding" turns). These
/// fire UNCONDITIONALLY of the proof NEG-arm: that transcript ran `cargo check`
/// AND `df` (proof tokens) yet still handed work to the operator and held. A
/// genuine hard limit is reported ONCE as an act-directive, never as a standing
/// instruction for the operator to run a command. Lower-cased substring match.
const HANDBACK_PHRASES: &[&str] = &[
    "owner — run",
    "owner - run",
    "owner must free",
    "owner must run",
    "owner-authorization anchor",
    "run in your terminal",
    "no agent action can",
    "only an external",
    "holding for",
    "holding until",
    "i'm holding",
    "i am holding",
];

/// Proof tokens whose presence means real work accompanies the prose — NEG-arm.
/// Any one present → the turn cited an artifact, so it is NOT pure gaming.
const PROOF_TOKENS: &[&str] = &[
    "git diff --stat",
    "cargo check",
    "cargo nextest",
    "status-update --status",
    "kavach db claim",
    "diff --stat",
    "files changed",
    "insertions(+)",
];

/// True when a turn-modified path is a REAL source/code file, not a doc. A
/// `.md`/`.txt`/`.mdx` write or anything under a `docs/` segment is a doc write
/// and does NOT count as real work for this gate.
fn is_real_source_write(path: &str) -> bool {
    let p = Path::new(path);
    let is_doc_ext = p.extension().is_some_and(|e| {
        e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("mdx") || e.eq_ignore_ascii_case("txt")
    });
    let in_docs_dir = p
        .components()
        .any(|c| c.as_os_str().eq_ignore_ascii_case("docs"));
    !is_doc_ext && !in_docs_dir
}

/// `true` when the message carries gaming/narration vocabulary OR an emoji
/// status-block line (a `✅`/`⏸`/`❌` on a `Phase:`/`State:`/`DONE` line).
fn has_gaming_language(lc: &str, raw: &str) -> bool {
    if GAMING_PHRASES.iter().any(|p| lc.contains(p)) {
        return true;
    }
    raw.lines().any(|line| {
        (line.contains('✅') || line.contains('⏸') || line.contains('❌'))
            && (line.contains("Phase:")
                || line.contains("State:")
                || line.to_uppercase().contains("DONE"))
    })
}

/// The block verdict (`Break`) when the three-condition AND holds; else `Continue`.
pub(crate) fn check(ctx: &mut StopCtx<'_>) -> ControlFlow<()> {
    // Re-entrant stop (hook already active) and explicit bypass: never fire.
    if ctx.input.stop_hook_active || std::env::var("KAVACH_DONE_GAMING_BYPASS").is_ok() {
        return ControlFlow::Continue(());
    }
    let raw = ctx.input.last_assistant_message.trim();
    let lc = raw.to_lowercase();

    // HANDBACK ARM (fires regardless of the proof NEG-arm and census): the turn
    // pushed the work to the operator ("Owner — run …") or is spinning a "Holding"
    // hold turn. This is the abolished surrender pattern; under a genuine infra
    // limit the answer is a ONE-SHOT act-directive (free your own scratch, run the
    // op via a runtime script), never a standing command for the operator. The
    // ENOSPC transcript proved the proof-token NEG-arm is not enough here — it ran
    // `cargo check`/`df` yet still handed back — so this arm is checked first and
    // does not consult PROOF_TOKENS.
    if HANDBACK_PHRASES.iter().any(|p| lc.contains(p)) {
        drop(kavach_hook::exit_stop_block(
            "[NO_HANDBACK] (non-surrenderable) This stop pushes work to the operator \
             (\"Owner — run …\" / \"owner must free\" / \"no agent action can\") or spins a \
             \"Holding\" turn. FORBIDDEN: you hold the shell, so YOU do the action. \
             If a real resource limit blocks you (disk full, missing tool, locked file): \
             (1) RECLAIM/REPAIR it yourself in-process — free your own regenerable build \
             scratch (`cargo clean`, delete idle `target/`, prune `~/.cache`/`/tmp`), \
             install the tool, break the stale lock; (2) if it is genuinely \
             secret/credential-bound, run the op via a runtime script (env in-process, \
             receipt out, value never in context); (3) then COMPLETE the blocked write and \
             resume. State a hard limit at most ONCE as a fact — never as a command for the \
             operator to run, and never as a repeated hold. The loop yields only to `Esc`.",
        ));
        return ControlFlow::Break(());
    }

    // Cond 1: gaming/narration language present.
    if !has_gaming_language(&lc, raw) {
        return ControlFlow::Continue(());
    }
    // NEG-arm: real proof accompanies the prose → not gaming.
    if PROOF_TOKENS.iter().any(|t| lc.contains(t)) {
        return ControlFlow::Continue(());
    }
    // Cond 3: a real (non-doc) source file moved this turn → real work happened.
    if ctx
        .session
        .files_modified_this_turn
        .iter()
        .any(|p| is_real_source_write(p))
    {
        return ControlFlow::Continue(());
    }
    // Cond 2: census must show runnable work remains. `None` (RPC outage) →
    // fail-OPEN: an unobservable board never wedges the loop.
    let runnable = match crate::gates::stop_dispatch::open_set_census(&ctx.session.project) {
        Some((runnable, _, _)) if runnable > 0 => runnable,
        _ => return ControlFlow::Continue(()),
    };

    drop(kavach_hook::exit_stop_block(&format!(
        "[DONE_GAMING] (non-surrenderable) {runnable} runnable card(s) remain and this turn \
         produced NO code/DB work — only narration (a status block / \"documentation pass\" / \
         \"vacuously complete\" / \"await features\"). Naming work done is NOT doing it \
         (§4a_describe_is_not_done). A phase is DONE only with a 3-witness artifact (rg file:line \
         + git diff --stat + cargo check exit 0) — never by redefinition (\"0 rows\", \"await \
         features\", \"gated\", \"doc pass\" are NOT done).\n\
         DO THIS TURN: the census is already read ({runnable} runnable) — claim ONE runnable card \
         (`kavach db claim ...`) and MUTATE code/DB to advance it; if it is genuinely \
         secret/credential-bound, run it via a runtime script (env in-process, receipt out, value \
         never in context). Drop the ✅/⏸ status decoration — imperative register only. \
         The loop ends on the user's `Esc`, never on a status doc."
    )));
    ControlFlow::Break(())
}

#[cfg(test)]
#[path = "done_gaming_test.rs"]
mod tests;

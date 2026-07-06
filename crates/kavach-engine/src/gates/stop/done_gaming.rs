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
use kavach_patterns::stop_vocab::DoneGamingVocab;
use super::shared::StopCtx;
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
        e.eq_ignore_ascii_case("md")
            || e.eq_ignore_ascii_case("mdx")
            || e.eq_ignore_ascii_case("txt")
    });
    let in_docs_dir = p
        .components()
        .any(|c| c.as_os_str().eq_ignore_ascii_case("docs"));
    !is_doc_ext && !in_docs_dir
}
/// `true` when the message carries gaming/narration vocabulary OR an emoji
/// status-block line (a `✅`/`⏸`/`❌` on a `Phase:`/`State:`/`DONE` line).
fn has_gaming_language(vocab: &DoneGamingVocab, lc: &str, raw: &str) -> bool {
    if vocab.has_gaming_phrase(lc) {
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
    // Vocab is DB-sourced (gate.done_gaming_vocab) with the compiled phrase floor
    // as fail-open default — the marker lists are DATA, hot-editable, not literals.
    let vocab = crate::gates::stop_dispatch::done_gaming_vocab_for(&ctx.session.project);
    // HANDBACK ARM fires regardless of the proof NEG-arm and census (the ENOSPC
    // transcript ran cargo check/df yet still handed back), so it is checked first
    // and does not consult PROOF_TOKENS. SOURCE: decision.done-gaming-vocab-dynamic.
    if vocab.has_handback_phrase(&lc) {
        drop(kavach_hook::exit_stop_block(
            "[NO_HANDBACK] (required) This stop hands work to the operator \
             (\"Owner — run …\" / \"owner must free\" / \"no agent action can\") or spins a \
             \"Holding\" turn. You hold the shell, so do the action yourself. \
             If a real resource limit blocks you (disk full, missing tool, locked file): \
             (1) reclaim or repair it yourself in-process — free your own regenerable build \
             scratch (`cargo clean`, delete idle `target/`, prune `~/.cache`/`/tmp`), \
             install the tool, break the stale lock; (2) if it is genuinely \
             secret/credential-bound, run the op via a runtime script (env in-process, \
             receipt out, value never in context); (3) then complete the blocked write and \
             resume. State a hard limit once as a fact, then keep working — rather than order the \
             operator to run it, repeat it, or hold. Only the user's `Esc` stops the loop.",
        ));
        return ControlFlow::Break(());
    }
    // Cond 1: gaming/narration language present.
    if !has_gaming_language(&vocab, &lc, raw) {
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
        "[DONE_GAMING] (required) {runnable} runnable card(s) remain and this turn \
         produced NO code/DB work — only narration (a status block / \"documentation pass\" / \
         \"vacuously complete\" / \"await features\"). Naming work done is NOT doing it \
         (§4a_describe_is_not_done). A phase is DONE only with a 3-witness artifact (rg file:line \
         + git diff --stat + cargo check exit 0) — NEVER redefine work as done; only \
         3-witness proof counts (\"0 rows\", \"await features\", \"gated\", \"doc pass\" are NOT done).\n\
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

//! Table-driven advisory-detector dispatch for the Stop gate (U5).
//!
//! CLOSE the "defined-but-never-enforced" loophole: the `kavach_chain`
//! `stop_signals` library holds 22 turn-end-stall detectors. WIRE each one into
//! a gate or NAME it in the deferral roster below — an unwired, unnamed detector
//! is dead code the `pub` export hides from `dead_code = deny`, and the stall it
//! catches sails straight through every Stop. See
//! `pattern.detector-verified-but-never-wired`.
//!
//! Each table entry maps a chain detector → a fix-first ADVISORY (queued for the
//! next turn's intent injector + recorded to the mistake ledger). ADVISORY tier
//! ONLY: the no-block policy forbids the Stop gate from ever halting the loop — it
//! NUDGES, it never stops. A regex-compile `Err` FAILS SAFE to "did not fire" (a
//! dead detector must never false-fire; the patterns are proven to compile by the
//! chain crate's own tests).
//!
//! WIRED (7) — invoked every Stop: `detect_continuation_menu` (in `stop.rs`),
//! and in `TABLE` below: `detect_permission_seek`, `detect_incomplete_work`,
//! `detect_remaining_phases`, `detect_unverified_code_claim`,
//! `detect_inference_as_evidence`, `detect_lazy_verification_claim`. The last
//! three are gated behind `wrote_this_turn` (a verification claim is meaningless
//! on a read-only turn).
//!
//! DEFERRED ROSTER (15) — each is EXPLICITLY held back; none is "etc." Wire it
//! the day its FP bound is proven < 1% in a test, then move it to WIRED. Until
//! then this roster IS the contract — a detector absent from BOTH lists is a bug,
//! not a judgment call. Reason codes: [HF]=high-FP pattern · [OV]=overlaps a
//! wired detector · [DUP]=duplicate definition, delete don't wire.
//!   `detect_value_gating`               [HF] broad "is this worth it" prose match
//!   `detect_user_report_dismissal`      [HF] fires on neutral "you reported X"
//!   `detect_strong_scope_ask`           [HF] overlaps `continuation_menu`'s NEG arm
//!   `detect_sycophancy`                 [HF] praise language is not a stall
//!   `detect_false_inability`            [HF] "I cannot" is often a true limit
//!   `detect_deferred_dismissal`         [HF] overlaps `strategic_deferral`
//!   `detect_self_imposed_limit`         [HF] "I'll keep this minimal" is often apt
//!   `detect_strategic_deferral`         [OV] strong candidate — wire after FP test
//!   `detect_unsolicited_reprioritization` [OV] candidate — needs NEG-arm FP proof
//!   `detect_parallel_system`            [OV] overlaps `incomplete_work`
//!   `detect_passive_info_request`       [OV] overlaps `permission_seek`
//!   `detect_summary_exit`               [OV] overlaps `remaining_phases`
//!   `detect_research_only_stop`         [OV] overlaps `incomplete_work` (write-gated)
//!   `detect_unwired_frontend_claim`     [HF] loose keyword-distance regex
//!   `detect_self_review_stop`           [DUP] defined twice; the `phase_b_review.rs`
//!                                          copy is an orphan (not mod-declared) —
//!                                          DELETE it, do not wire.

use kavach_chain::stop_signals;

/// One detector in the dispatch table: a chain predicate, the advisory it emits
/// when it fires, and the mistake-ledger metadata. `needs_write` gates the
/// verification-claim detectors so they fire only when a file was actually
/// written this turn (a verification claim is meaningless on a read-only turn).
struct Entry {
    /// The chain detector predicate. Returns `Ok(true)` when the stall fires.
    detect: fn(&str) -> Result<bool, regex::Error>,
    /// Only fire if a file was written this turn (verification-claim guard).
    needs_write: bool,
    /// Mistake-ledger gate key (the learning-loop bucket).
    gate: &'static str,
    /// Mistake-ledger banned-sample description.
    banned: &'static str,
    /// Mistake-ledger correct-action description.
    correct: &'static str,
    /// The fix-first advisory queued for the next turn's intent injector.
    advisory: &'static str,
}

/// The dispatch table. Order is irrelevant (every firing entry queues its own
/// advisory; advisories never block). Each `advisory` points the model back at
/// the kavach DB / verify gate — never at the user.
const TABLE: &[Entry] = &[
    Entry {
        detect: stop_signals::detect_permission_seek,
        needs_write: false,
        gate: "permission_seek_at_stop",
        banned: "asked the user permission to proceed/continue while [AUTO_CONTINUE] already commanded continuation",
        correct: "do NOT ask — query the kavach DB (kanban + roadmap/decision), claim the next card, START it THIS turn",
        advisory: "[PERMISSION_SEEK] last turn asked the user for permission to proceed/continue — but the loop directive ([AUTO_CONTINUE]/[ALL_BLOCKED]) already commanded the next move. Do NOT ask: query the kavach DB (kanban + `kavach db query --category roadmap`/`--category decision`), claim the next runnable card, and START it THIS turn. Asking to proceed is the forbidden deferral (global CLAUDE.md §autonomous_loop §act_not_narrate).",
    },
    Entry {
        detect: stop_signals::detect_incomplete_work,
        needs_write: true,
        gate: "incomplete_work_at_stop",
        banned: "announced incomplete work and stopped without finishing or filing a blocker",
        correct: "complete the work THIS turn, or file a roadmap card naming the exact blocker",
        advisory: "[INCOMPLETE_WORK] last turn announced incomplete work and stopped. The loop expects COMPLETION. Do NOT narrate what is left: check the kavach DB, and if the work is runnable, FINISH it this turn (read/write/verify); if genuinely blocked, file a card naming the exact blocker. Never leave work hanging in prose.",
    },
    Entry {
        detect: stop_signals::detect_remaining_phases,
        needs_write: false,
        gate: "remaining_phases_at_stop",
        banned: "named remaining phases/steps then stopped instead of executing them",
        correct: "start the next named step THIS turn, or state the board is genuinely empty as a fact",
        advisory: "[REMAINING_PHASES] last turn named remaining phases/steps but did not START them — naming the next work is a TRIGGER to DO it, not a sign-off (global CLAUDE.md §4a_describe_is_not_done). Check the kavach DB, claim the next task, and START it THIS turn. If all work is done, STATE that as a fact — never end on a list of remaining items.",
    },
    Entry {
        detect: stop_signals::detect_unverified_code_claim,
        needs_write: true,
        gate: "unverified_code_claim_at_stop",
        banned: "claimed code is ready/done/working without citing 3-witness artifacts (rg + git diff + cargo check)",
        correct: "run the three-witness gate (rg + git diff --stat + cargo check exit 0) and cite the artifacts, or drop the claim",
        advisory: "[UNVERIFIED_CODE] last turn claimed code is ready/done/working without proof. Run the three witnesses THIS turn: `rg <pattern> <file>` (present), `git diff --stat` (landed), `cargo check --workspace` exit 0 (compiles) — and cite the file:line, or state the exact remaining verify step and continue it.",
    },
    Entry {
        detect: stop_signals::detect_inference_as_evidence,
        needs_write: true,
        gate: "inference_as_evidence_at_stop",
        banned: "asserted a result from inference (\"it compiled, so it works\") with no observed artifact",
        correct: "verify the SEMANTIC effect (the change ran, the row landed) and cite it, not merely the absence of an error",
        advisory: "[INFERENCE_AS_EVIDENCE] last turn inferred a result (\"it compiled / the call returned, so it works\") instead of observing it. \"It compiled\" does NOT imply \"it works\"; \"the call returned\" does NOT imply \"the effect happened\" (global CLAUDE.md §evidence_over_inference). Verify the actual effect THIS turn and cite the artifact.",
    },
    Entry {
        detect: stop_signals::detect_lazy_verification_claim,
        needs_write: true,
        gate: "lazy_verification_claim_at_stop",
        banned: "claimed verification without the 3-witness artifacts",
        correct: "produce the rg hit + git diff --stat + cargo check exit 0, or drop the verified claim",
        advisory: "[LAZY_VERIFICATION] last turn claimed verification without the artifacts. A CLEAN verdict is noise unless it cites the leaf it reached (global CLAUDE.md §verdict_needs_leaf_evidence). Produce the three witnesses THIS turn (rg hit + git diff --stat + cargo check exit 0) and cite them, or drop the claim.",
    },
];

/// Run the advisory-detector table over the final message. For each firing
/// entry, record a mistake-ledger row (learning loop) and queue a harness-neutral
/// pending advisory (re-surfaces at the top of the next turn, before the next
/// message). Pure side-effect: never blocks, never returns a stop verdict.
///
/// `wrote_this_turn` gates the verification-claim entries. A detector whose
/// pattern fails to compile is treated as "did not fire" (fail-safe).
pub(super) fn run(session: &mut kavach_session::SessionState, msg: &str, wrote_this_turn: bool) {
    for entry in TABLE {
        if entry.needs_write && !wrote_this_turn {
            continue;
        }
        if (entry.detect)(msg).unwrap_or(false) {
            drop(kavach_session::record_mistake(&kavach_session::Mistake {
                project: &session.project,
                gate: entry.gate,
                banned_sample: entry.banned,
                correct_action: entry.correct,
                turn: session.turn_count,
            }));
            session.queue_pending_advisory(entry.advisory);
        }
    }
}

#[cfg(test)]
#[path = "advisory_detectors_test.rs"]
mod tests;

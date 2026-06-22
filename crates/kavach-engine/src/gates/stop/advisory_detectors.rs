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
//! next turn's intent injector + recorded to the mistake ledger). The advisory is
//! always queued; ADDITIONALLY, the gate keys in `ARGUE_GATES` / `HANDBACK_GATES` /
//! the research key set the `StallSignals` that give `clean_exit` refuse-stop TEETH
//! (a prose nudge against sycophancy/disobedience is proven NOT to work — only
//! dynamic behavioral GATING does: arxiv.org/pdf/2604.00478, 2604.02423). All teeth
//! are breaker-bounded so a turn that genuinely cannot proceed force-allows after N.
//! A regex-compile `Err` FAILS SAFE to "did not fire" (a dead detector must never
//! false-fire; the patterns are proven to compile by the chain crate's own tests).
//!
//! WIRED (12) — invoked every Stop: `detect_continuation_menu` (in `stop.rs`),
//! and in `TABLE` below: `detect_permission_seek`, `detect_incomplete_work`,
//! `detect_remaining_phases`, `detect_unverified_code_claim`,
//! `detect_inference_as_evidence`, `detect_lazy_verification_claim`, the four
//! action-driven imperatives `detect_completion_without_witnesses`,
//! `detect_decision_not_persisted`, `detect_verdict_without_citation`,
//! `detect_claim_without_research` (Phase E — claim-without-action), and
//! `semantic_deferral_backstop` (the `classify_semantic_deferral` adapter — an
//! LLM-judge-shaped semantic backstop for PARAPHRASED handoffs the lexical
//! `detect_strategic_deferral` regex misses; FP-bounded by its own
//! `CoveredByRegex`/`PRESENT_ACTION` arms, proven in
//! `phase_a_semantic_deferral_test.rs`). The verification-claim three are gated
//! behind `wrote_this_turn` (a verification claim is meaningless on a read-only
//! turn).
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

use kavach_chain::stop_signals::{
    self, SemanticDeferral, classify_semantic_deferral, detect_user_report_dismissal,
    detect_value_gating,
};

/// Backstop adapter: fire ONLY on a paraphrased handoff the lexical
/// `detect_strategic_deferral` regex missed. `CoveredByRegex`/`Clear` → false, so
/// this never double-counts a regex hit and never fires on an actively-working
/// turn (the classifier's `PRESENT_ACTION` arm negates it).
fn semantic_deferral_backstop(msg: &str) -> Result<bool, regex::Error> {
    Ok(classify_semantic_deferral(msg)? == SemanticDeferral::ParaphrasedHandoff)
}

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
        advisory: "[PERMISSION_SEEK] last turn asked the user for permission to proceed/continue — but the loop directive ([AUTO_CONTINUE]/[BLOCKER_WALK]) already commanded the next move. Do NOT ask: query the kavach DB (kanban + `kavach db query --category roadmap`/`--category decision`), claim the next runnable card, and START it THIS turn. Asking to proceed is the forbidden deferral (global CLAUDE.md §autonomous_loop §act_not_narrate).",
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
    Entry {
        detect: stop_signals::detect_completion_without_witnesses,
        needs_write: true,
        gate: "completion_without_witnesses_at_stop",
        banned: "declared the task done/complete/shipped without citing the three witnesses (rg artifact + git diff --stat + cargo/nextest)",
        correct: "produce the three witnesses THIS turn (artifact via rg, diff via git diff --stat, build via cargo check/nextest) and cite them, or state the exact remaining step",
        advisory: "[THREE_WITNESS] last turn narrated completion (done/complete/shipped/landed) without the three proofs. 'Done' requires THREE witnesses, not prose: (1) the artifact exists (`rg`), (2) the diff landed (`git diff --stat`), (3) the build passes (`cargo check`/`nextest`). Narrating completion is not completing it (global CLAUDE.md §three-witness). Produce + cite all three THIS turn, or name the exact remaining step and continue it.",
    },
    Entry {
        detect: stop_signals::detect_decision_not_persisted,
        needs_write: false,
        gate: "decision_not_persisted_at_stop",
        banned: "announced a settled design/decision/approach but did not persist it to the kavach DB the same turn",
        correct: "write the decision row THIS turn (`kavach db write --category decision`) — choice + source + one-line why",
        advisory: "[PERSIST_NOW] last turn settled a design choice/decision/approach but did not persist it. A settled decision is written the SAME turn — to the kavach DB (`kavach db write --category decision`: choice + source + one-line why), not into prose that evaporates (global CLAUDE.md §persist-decisions). A mistake corrected twice was never persisted once. Write the row THIS turn.",
    },
    Entry {
        detect: stop_signals::detect_verdict_without_citation,
        needs_write: false,
        gate: "verdict_without_citation_at_stop",
        banned: "issued a clean/wired/safe/no-defect verdict without citing the file:line read to reach it",
        correct: "open the entry->logic call path and cite the file:line you read, or drop the verdict",
        advisory: "[CITE_EVIDENCE] last turn issued a clean/wired/safe/no-defect verdict with no `file:line`. A verdict must name the leaf you READ to reach it (global CLAUDE.md §verdicts-cite-evidence). Open the entry->logic call path, cite the `file:line` THIS turn, or drop the verdict.",
    },
    Entry {
        detect: stop_signals::detect_claim_without_research,
        needs_write: false,
        gate: "claim_without_research_at_stop",
        banned: "asserted a current-knowledge fact (latest version/API/pricing/support) with no source URL or [RESEARCH]/SOURCE marker",
        correct: "research a real source and cite its URL THIS turn before the claim stands; no source -> no claim",
        advisory: "[RESEARCH_FIRST] last turn asserted a current-knowledge fact (latest/version/API/pricing/supports) from memory, with no source URL. Tabula rasa: the weights are stale, the web is truth (global CLAUDE.md §internet-first). Research a real source and cite its URL THIS turn — no source, no claim.",
    },
    Entry {
        detect: semantic_deferral_backstop,
        needs_write: false,
        gate: "semantic_deferral_at_stop",
        banned: "paraphrased a handoff (\"good stopping point\", \"handing the rest off\", \"as far as makes sense\") the lexical deferral regex missed",
        correct: "do NOT hand off — query the kavach DB, claim the next runnable card, and START it THIS turn",
        advisory: "[SEMANTIC_DEFERRAL] last turn read as a paraphrased handoff the lexical deferral regex did not catch (\"natural stopping point\", \"handing the remainder off\", \"as far as it makes sense\"). A paraphrase is still a deferral (global CLAUDE.md §autonomous_loop §4a_describe_is_not_done). Do NOT hand work off in prose: check the kavach DB, claim the next runnable card, and START it THIS turn. If the board is genuinely empty, state that as a fact.",
    },
    Entry {
        detect: detect_user_report_dismissal,
        needs_write: false,
        gate: "argued_with_user_at_stop",
        banned: "dismissed what the user reported/asked (\"you reported X\", \"this is expected behavior\", \"working as designed\") instead of obeying the user's stated intent",
        correct: "RE-READ the user's exact words, UNDERSTAND the intent behind them, and OBEY it THIS turn — never refute the user with a strawman; if a fact is in dispute, WebSearch and cite a URL, never assert from memory",
        advisory: "[ARGUED_WITH_USER] last turn dismissed/refuted what the user reported or asked instead of obeying the stated intent. RE-READ the user's exact words. UNDERSTAND the intent. OBEY it THIS turn (global CLAUDE.md §No-Fence). The user's instruction is a TRIGGER to ACT, not a position to argue with. If a factual claim is in dispute, WebSearch and cite the URL — NEVER answer from training weights (§internet-first, §No-Hallucination). Do the mandated action, then stop.",
    },
    Entry {
        detect: detect_value_gating,
        needs_write: false,
        gate: "value_gated_user_request_at_stop",
        banned: "argued the user's request adds no value / is not worth it / should wait, instead of building what the user asked for",
        correct: "BUILD what the user asked THIS turn — value-gating the user's own request is the forbidden argue-not-obey; if scope is unclear, ask ONE tight clarifying question, never refuse",
        advisory: "[VALUE_GATED_REQUEST] last turn argued the user's request adds no value / is not worth it / can wait — value-gating the user's OWN explicit request is argue-not-obey (global CLAUDE.md §No-Fence, §No-Fluff). BUILD what the user asked THIS turn. If genuinely uncertain about scope, ask ONE tight clarifying question — never refuse or defer.",
    },
];

/// Which refuse-stop-eligible stall classes fired this turn. ADVISORY firing is a
/// pure side-effect of `run`; these flags let `clean_exit` give two laws TEETH
/// (refuse the stop, breaker-bounded) instead of an advisory the model can ignore.
#[derive(Default)]
pub(super) struct StallSignals {
    /// `detect_claim_without_research` fired — an unsourced current-knowledge claim
    /// (internet-first law). Refused unconditionally (breaker-bounded).
    pub research_unsourced: bool,
    /// A handback / permission-menu / name-then-stop / paraphrased-handoff fired —
    /// the argue-not-obey class. Refused ONLY when census proves dispatchable work
    /// (`roadmap_todos_remain`), breaker-bounded. This is the generalized
    /// disobedience signal the narrow lexical `disobedience_guard` was blind to.
    pub handback_or_menu: bool,
    /// `detect_user_report_dismissal` or `detect_value_gating` fired — the turn
    /// ARGUED WITH / refuted / value-gated the user's own stated request instead of
    /// obeying the intent. Refused unconditionally (breaker-bounded), census-
    /// INDEPENDENT: arguing with the user is wrong whether or not the board has todos.
    /// This is the anti-sycophancy teeth (SOURCE: arxiv.org/pdf/2604.00478 dynamic
    /// behavioral gating; a prose "don't argue" nudge is proven NOT to work).
    pub argued_with_user: bool,
}

/// Gate keys whose firing means the turn ended on a HANDBACK or PERMISSION-MENU
/// rather than the next action — the argue-not-obey class. Distinct from the
/// verification-claim entries (which only mean "prove it", not "you disobeyed").
const HANDBACK_GATES: &[&str] = &[
    "permission_seek_at_stop",
    "remaining_phases_at_stop",
    "incomplete_work_at_stop",
    "semantic_deferral_at_stop",
];

/// Gate keys whose firing means the turn ARGUED WITH the user — refuted what the
/// user reported, or value-gated the user's own request — rather than obeying the
/// stated intent. Distinct from HANDBACK (which defers): these turns actively push
/// BACK. Refuse-stop is census-INDEPENDENT for this class (arguing with the user is
/// wrong whether or not a roadmap todo remains). SOURCE: decision.engine.anti-argue-block.
const ARGUE_GATES: &[&str] = &["argued_with_user_at_stop", "value_gated_user_request_at_stop"];

/// Run the advisory-detector table over the final message. For each firing
/// entry, record a mistake-ledger row (learning loop) and queue a harness-neutral
/// pending advisory (re-surfaces at the top of the next turn, before the next
/// message). Pure side-effect: never blocks, never returns a stop verdict.
///
/// `wrote_this_turn` gates the verification-claim entries. A detector whose
/// pattern fails to compile is treated as "did not fire" (fail-safe).
///
/// Returns the `StallSignals` the caller (`clean_exit`) uses to REFUSE the stop:
/// `research_unsourced` (internet-first teeth) and `handback_or_menu` (argue-not-obey
/// teeth) — both breaker-bounded so a turn that genuinely cannot act force-allows.
pub(super) fn run(
    session: &mut kavach_session::SessionState,
    msg: &str,
    wrote_this_turn: bool,
) -> StallSignals {
    let mut signals = StallSignals::default();
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
            if entry.gate == "claim_without_research_at_stop" {
                signals.research_unsourced = true;
            }
            if HANDBACK_GATES.contains(&entry.gate) {
                signals.handback_or_menu = true;
            }
            if ARGUE_GATES.contains(&entry.gate) {
                signals.argued_with_user = true;
            }
        }
    }
    signals
}

#[cfg(test)]
#[path = "advisory_detectors_test.rs"]
mod tests;

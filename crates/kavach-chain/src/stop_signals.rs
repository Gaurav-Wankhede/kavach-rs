//! NLU regex-backed semantic stop-signal classifiers.
//!
//! Per-concept morphological regex predicates replacing flat `.contains()` literal
//! arrays across ~32 detectors. Each detector is a public function; implementation
//! split into micro-modules under `stop_signals/` to honor the ≤100-LOC file rule.

pub use self::phase_a::{
    SemanticDeferral, classify_semantic_deferral, detect_continuation_menu,
    detect_self_imposed_limit, detect_strategic_deferral, detect_strong_scope_ask,
    detect_unsolicited_reprioritization, detect_value_gating,
};

pub use self::phase_b::{
    detect_false_inability, detect_incomplete_work, detect_parallel_system,
    detect_passive_info_request, detect_remaining_phases, detect_sycophancy,
};

pub use self::phase_c::{
    detect_deferred_dismissal, detect_permission_seek, detect_research_only_stop,
    detect_summary_exit, detect_user_report_dismissal,
};

pub use self::phase_d::{
    detect_inference_as_evidence, detect_lazy_verification_claim, detect_self_review_stop,
    detect_unverified_code_claim, detect_unwired_frontend_claim,
};

pub use self::phase_e_action::{
    detect_claim_without_research, detect_completion_without_witnesses,
    detect_decision_not_persisted, detect_verdict_without_citation,
};

mod signal;

// Phase A leaf modules
mod phase_a;
mod phase_a_deferral;
mod phase_a_limits;
mod phase_a_menu;
mod phase_a_semantic_deferral;
mod phase_a_value;

// Phase B leaf modules
mod phase_b;
mod phase_b_lex1;
mod phase_b_lex2;

// Phase C leaf modules
mod phase_c;
mod phase_c_lex;
mod phase_c_permission;
mod phase_c_research;

// Phase D leaf modules
mod phase_d;
mod phase_d_lex1;
mod phase_d_lex2;
mod phase_d_multi;

// Phase E — action-driven imperatives (completion/decision/verdict/research)
mod phase_e_action;

#[cfg(test)]
#[path = "stop_signals_test.rs"]
#[cfg(test)]
#[path = "stop_signals_test.rs"]
mod tests;
//! The check orchestrator: exemptions → trigger scan → auto-inject / block.
use super::decision::load_prior_decision;
use super::outcome::AlgoGuardOutcome;
use super::strip::strip_string_literals;
use super::triggers::ALGO_TRIGGERS;

const BLOCK_MSG: &str = "ALGO_HUNTER_REQUIRED: this write introduces algorithmic or data-structure logic.\n\
     \n\
     Run /arch BEFORE this write. Do NOT write any provenance comment.\n\
     1. Invoke /arch — research the problem class, compare ≥3 candidates with current benchmarks.\n\
     2. Persist the choice as a kavach decision row (choice + source link + rationale). Comments waste tokens; the row is the record.\n\
     3. Retry this Write — the gate approves on the recorded /arch invocation.";

/// Check whether the write requires prior `/arch` invocation.
///
/// - `algo_satisfied`: true if the hunter was invoked this session.
/// - `project_slug`: queries kavach-db for prior decisions (auto-inject path).
pub(crate) fn check(
    file_path: &str,
    content: &str,
    algo_satisfied: bool,
    project_slug: &str,
) -> AlgoGuardOutcome {
    if !std::path::Path::new(file_path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
    {
        return AlgoGuardOutcome::Allow;
    }
    // Test files are exempt — algorithm choices in tests don't need the hunter.
    let test_patterns = ["_tests.rs", "_test.rs", "tests/", "test_"];
    if test_patterns.iter().any(|p| file_path.contains(p)) {
        return AlgoGuardOutcome::Allow;
    }
    if content.is_empty() || algo_satisfied {
        return AlgoGuardOutcome::Allow;
    }
    let stripped = strip_string_literals(content);
    let Some(trigger_kw) = ALGO_TRIGGERS.iter().find(|kw| stripped.contains(*kw)) else {
        return AlgoGuardOutcome::Allow;
    };
    // Auto-inject path: look for a prior decision in kavach-db.
    if let Some(ctx) = load_prior_decision(project_slug, trigger_kw) {
        return AlgoGuardOutcome::AutoInject(ctx);
    }
    AlgoGuardOutcome::Block(BLOCK_MSG.into())
}

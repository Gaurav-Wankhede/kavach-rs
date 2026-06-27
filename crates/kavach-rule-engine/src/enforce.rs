//! Tiered skill enforcement: critical must all pass, advisory any-one.
use crate::file_matcher::FileMatchResult;
#[derive(Debug)]
#[expect(
    clippy::exhaustive_enums,
    reason = "enum constructed at RPC handler boundary"
)]
pub enum EnforcementDecision {
    Allowed,
    Blocked {
        missing_critical: Vec<String>,
        missing_advisory: Vec<String>,
    },
}
/// Check if invoked skills satisfy the file match requirements.
///
/// Critical skills: ALL must be invoked.
/// Advisory skills: at least ONE must be invoked (any-one pool).
/// Exception: if all critical are satisfied, advisory is waived.
#[must_use]
pub fn check_enforcement(
    matches: &FileMatchResult,
    invoked_skills: &[String],
) -> EnforcementDecision {
    if !matches.has_matches() {
        return EnforcementDecision::Allowed;
    }
    let missing_critical: Vec<String> = matches
        .critical
        .iter()
        .filter(|s| !invoked_skills.iter().any(|inv| inv == *s))
        .cloned()
        .collect();
    // All critical present AND all satisfied → advisory waived.
    if !matches.critical.is_empty() && missing_critical.is_empty() {
        return EnforcementDecision::Allowed;
    }
    // Advisory: at least one must be invoked (unless none required).
    let advisory_satisfied = matches.advisory.is_empty()
        || matches
            .advisory
            .iter()
            .any(|s| invoked_skills.iter().any(|inv| inv == s));
    if missing_critical.is_empty() && advisory_satisfied {
        return EnforcementDecision::Allowed;
    }
    let missing_advisory = if advisory_satisfied {
        vec![]
    } else {
        matches.advisory.clone()
    };
    EnforcementDecision::Blocked {
        missing_critical,
        missing_advisory,
    }
}
/// Format a block reason message from an enforcement decision.
#[must_use]
pub fn format_block_reason(decision: &EnforcementDecision, file_path: &str) -> String {
    match decision {
        EnforcementDecision::Allowed => String::new(),
        EnforcementDecision::Blocked {
            missing_critical,
            missing_advisory,
        } => {
            let mut parts = Vec::new();
            if !missing_critical.is_empty() {
                let names = missing_critical.join(", ");
                parts.push(format!(
                    "File {file_path} requires critical skill(s): [{names}]"
                ));
            }
            if !missing_advisory.is_empty() {
                let names = missing_advisory
                    .iter()
                    .map(|s| format!("/{s}"))
                    .collect::<Vec<_>>()
                    .join(" or ");
                parts.push(format!("Invoke at least one: {names}"));
            }
            format!("SKILL VIOLATION: {}", parts.join(". "))
        }
    }
}
#[cfg(test)]
#[path = "enforce_tests.rs"]
mod tests;

// ARCH: CounterfactualAnalysis — "What if this change were NOT made?"
// PATTERN: counterfactual | SCOPE: pre_write | CAP: AP | SEARCHED: 2026-04
// Per verification gate research: detect unnecessary complexity before it lands.

/// Counterfactual advisory: detect patterns that suggest unnecessary complexity.
/// Returns None if code looks necessary, Some(advisory) if speculative/premature.
pub(super) fn counterfactual_advisory(content: &str) -> Option<String> {
    let mut issues: Vec<&str> = Vec::new();

    if content.contains("impl<T>") && content.contains("where T:") && content.len() < 500 {
        issues.push("Generic impl on small code — is the abstraction needed now?");
    }
    if content.contains("feature_flag") || content.contains("FEATURE_") {
        issues.push("Feature flag detected — is this needed for current task?");
    }
    if content.contains("todo!()") || content.contains("unimplemented!()") {
        issues.push("Placeholder detected — implement fully or don't add yet");
    }
    if content.contains("Factory") && content.contains("Builder") {
        issues.push("Factory+Builder pattern — is this complexity warranted?");
    }
    if content.contains("_deprecated") || content.contains("// DEPRECATED") {
        issues.push("Deprecated marker — can you remove instead of deprecate?");
    }

    if issues.is_empty() {
        return None;
    }
    let mut advisory = String::from("[COUNTERFACTUAL] What if this were NOT made?\n");
    for issue in issues {
        advisory.push_str("  - ");
        advisory.push_str(issue);
        advisory.push('\n');
    }
    advisory.push_str("Principle: The right complexity is what the task requires.");
    Some(advisory)
}

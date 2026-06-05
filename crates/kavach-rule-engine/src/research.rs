//! Research gate enforcement: ensure research before code changes.

use crate::context::EvalContext;
use crate::result::{RuleAction, RuleResult};
use kavach_config::requires_research;

/// Check if research was done before code changes.
/// Returns a Block result if research is required but not done.
#[must_use]
pub fn check_research_gate(ctx: &EvalContext) -> Option<RuleResult> {
    if ctx.research_done || !ctx.is_write_tool() || !ctx.is_code_target() {
        return None;
    }
    if !requires_research(&ctx.prompt) {
        return None;
    }
    let queries = build_suggested_queries(ctx);
    let reason = format!(
        "Research required before code changes. Suggested queries: {}",
        queries.join(", ")
    );
    Some(RuleResult {
        action: RuleAction::Block,
        rule_name: "research_gate".into(),
        reason,
        skill_name: None,
        severity: 8,
    })
}

/// Build suggested search queries based on file extension and prompt.
#[must_use]
pub fn build_suggested_queries(ctx: &EvalContext) -> Vec<String> {
    let prompt = &ctx.prompt;
    if let Some(ref fp) = ctx.file_path {
        let ext = fp.rsplit('.').next().unwrap_or("");
        let suffix = match ext {
            "rs" => "Rust docs.rs 2026",
            "go" => "Go pkg.go.dev 2026",
            "ts" | "tsx" => "TypeScript MDN 2026",
            "py" => "Python docs 2026",
            _ => return vec![format!("{prompt} {ext} docs 2026")],
        };
        return vec![format!("{prompt} {suffix}")];
    }
    vec![format!("{prompt} latest docs 2026")]
}

/// Validate that research sources are present and contain valid URLs.
#[must_use]
pub fn validate_research_sources(sources: &[String]) -> RuleResult {
    if sources.is_empty() {
        return RuleResult::warn("research_sources", "No research sources provided", 6);
    }
    let valid = sources
        .iter()
        .filter(|s| s.starts_with("http") && s.len() > 10)
        .count();
    if valid == 0 {
        return RuleResult::warn("research_sources", "No valid URLs in research sources", 5);
    }
    RuleResult::allow("research_sources", "Research sources validated")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_research_done_skips() {
        let ctx = EvalContext::new("Write", "fix bug")
            .with_file("src/main.rs")
            .with_research(true);
        assert!(check_research_gate(&ctx).is_none());
    }

    #[test]
    fn test_non_write_skips() {
        let ctx = EvalContext::new("Read", "fix bug").with_file("src/main.rs");
        assert!(check_research_gate(&ctx).is_none());
    }

    #[test]
    fn test_build_queries_rust() {
        let ctx = EvalContext::new("Write", "add axum handler").with_file("src/handler.rs");
        let q = build_suggested_queries(&ctx);
        assert!(!q.is_empty());
        assert!(q.first().is_some_and(|first| first.contains("Rust")));
    }

    #[test]
    fn test_validate_sources_empty() {
        let r = validate_research_sources(&[]);
        assert_eq!(r.action, RuleAction::Warn);
    }

    #[test]
    fn test_validate_sources_valid() {
        let sources = vec!["https://docs.rs/axum/latest".to_owned()];
        let r = validate_research_sources(&sources);
        assert_eq!(r.action, RuleAction::Allow);
    }
}

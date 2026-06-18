use std::fmt::Write as _;
use std::path::PathBuf;

use kavach_rule_engine::context::SessionPhase;
use kavach_rule_engine::{EvalContext, RuleAction, RuleEngine, RuleResult};
use kavach_types::HookInput;

/// Evaluate rules from skills directory against hook input.
///
/// Returns empty Vec if skills directory is legitimately absent (no custom rules).
/// If directory exists but cannot be read, still proceeds (engine logs warnings).
#[must_use]
pub(crate) fn evaluate_rules(input: &HookInput) -> Vec<RuleResult> {
    let Some(skills_dir) = skills_dir() else {
        return Vec::new();
    };

    // Check if the directory exists. If it doesn't, that's fine — no custom rules.
    if !skills_dir.exists() {
        return Vec::new();
    }

    // Directory exists: load and evaluate. The engine will log warnings on parse errors.
    let mut engine = RuleEngine::new(skills_dir);
    engine.load_skills();

    let session = kavach_session::get_or_create_session();
    let tool_name = &input.tool_name;
    let prompt = input.get_string("prompt");
    let file_path = input.get_string("file_path");
    let content = input.get_string("content");

    let phase = match session.context_phase.as_str() {
        "mid" => SessionPhase::Mid,
        "late" => SessionPhase::Late,
        "critical" => SessionPhase::Critical,
        _ => SessionPhase::Early,
    };

    let mut ctx = EvalContext::new(tool_name, prompt)
        .with_research(session.research_done)
        .with_phase(phase);
    if !file_path.is_empty() {
        ctx = ctx.with_file(file_path);
    }
    if !content.is_empty() {
        ctx = ctx.with_content(content);
    }

    engine.evaluate(&ctx)
}

/// Format rule results as TOON context string.
#[must_use]
pub(crate) fn results_to_context(results: &[RuleResult]) -> String {
    if results.is_empty() {
        return String::new();
    }
    let mut out = String::from("[RULE_ENGINE]\n");
    for r in results {
        let action = match r.action {
            RuleAction::Block => "BLOCK",
            RuleAction::Warn => "WARN",
            RuleAction::Modify => "MODIFY",
            RuleAction::Allow => "ALLOW",
            _ => "UNKNOWN",
        };
        writeln!(
            out,
            "{action}: {} (sev={}) — {}",
            r.rule_name, r.severity, r.reason
        )
        .ok();
    }
    out
}

fn skills_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".claude").join("skills"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_results_to_context_empty() {
        assert_eq!(results_to_context(&[]), "");
    }

    #[test]
    fn test_results_to_context_formatted() {
        let results = vec![RuleResult::warn("stub_check", "found todo", 7)];
        let toon = results_to_context(&results);
        assert!(toon.contains("[RULE_ENGINE]"));
        assert!(toon.contains("WARN: stub_check"));
        assert!(toon.contains("sev=7"));
    }

    #[test]
    fn test_skills_dir() {
        if let Some(dir) = skills_dir() {
            assert!(dir.ends_with("skills"));
        }
    }
}

//! Rule matching: find applicable skills for a given context.

use crate::context::EvalContext;
use kavach_config::get_skills_by_priority;
use kavach_rule_ast::SkillDefinition;

/// A matched rule with its source skill and match score.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct MatchedRule {
    pub skill: SkillDefinition,
    pub score: u32,
    pub matched_on: String,
}

/// Find all skill definitions that apply to the given context.
/// Returns matches sorted by descending score.
#[must_use]
pub fn match_rules(skills: &[SkillDefinition], ctx: &EvalContext) -> Vec<MatchedRule> {
    let prompt_lower = ctx.prompt.to_lowercase();
    let tool_lower = ctx.tool_name.to_lowercase();
    let mut matches: Vec<MatchedRule> = Vec::new();
    for skill in skills {
        let (mut score, mut matched_on) = (0u32, String::new());
        for trigger in &skill.metadata.triggers {
            let tl = trigger.to_lowercase();
            if prompt_lower.contains(&tl) {
                score = score.saturating_add(10);
                matched_on = format!("prompt:{trigger}");
            }
            if tool_lower.contains(&tl) {
                score = score.saturating_add(5);
                matched_on = format!("tool:{trigger}");
            }
        }
        if let Some(ref fp) = ctx.file_path
            && match_file_to_skill(fp, &skill.metadata.name)
        {
            score = score.saturating_add(8);
            matched_on = format!("file:{fp}");
        }
        if score > 0 {
            matches.push(MatchedRule {
                skill: skill.clone(),
                score,
                matched_on,
            });
        }
    }
    matches.sort_by_key(|m| std::cmp::Reverse(m.score));
    matches
}

/// Check if a file path is relevant to a skill by name/keyword overlap.
fn match_file_to_skill(file_path: &str, skill_name: &str) -> bool {
    let fp_lower = file_path.to_lowercase();
    skill_name.split(&['-', '_'][..]).any(|part| {
        let p = part.to_lowercase();
        p.len() > 2 && fp_lower.contains(&p)
    })
}

/// Get config-based skill keywords and check if context matches any.
#[must_use]
pub fn match_config_skills(ctx: &EvalContext) -> Vec<String> {
    let prompt_lower = ctx.prompt.to_lowercase();
    get_skills_by_priority()
        .into_iter()
        .filter(|s| s.keywords.iter().any(|k| prompt_lower.contains(k)))
        .map(|s| s.name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kavach_rule_ast::{ResearchGate, SkillDefinition, SkillMetadata};

    fn test_skill(name: &str, triggers: &[&str]) -> SkillDefinition {
        SkillDefinition {
            metadata: SkillMetadata {
                name: name.into(),
                description: "test".into(),
                protocol: "SP/1.0".into(),
                triggers: triggers.iter().map(ToString::to_string).collect(),
                file_patterns: Vec::new(),
                priority: kavach_rule_ast::SkillPriority::default(),
            },
            research_gate: ResearchGate {
                mandatory: true,
                rule: "search first".into(),
            },
        }
    }

    #[test]
    fn test_match_by_prompt() {
        let skills = vec![test_skill("rust", &["rust", "cargo"])];
        let ctx = EvalContext::new("Write", "add rust handler");
        let m = match_rules(&skills, &ctx);
        assert_eq!(m.len(), 1);
        assert!(m.first().is_some_and(|first| first.score >= 10));
    }

    #[test]
    fn test_no_match() {
        let skills = vec![test_skill("python", &["python", "pip"])];
        let ctx = EvalContext::new("Write", "add rust handler");
        assert!(match_rules(&skills, &ctx).is_empty());
    }

    #[test]
    fn test_file_match() {
        let skills = vec![test_skill("rust-axum", &[])];
        let ctx = EvalContext::new("Write", "fix").with_file("src/axum_handler.rs");
        let m = match_rules(&skills, &ctx);
        assert_eq!(m.len(), 1);
        assert!(
            m.first()
                .is_some_and(|first| first.matched_on.starts_with("file:"))
        );
    }
}

//! Generates `SkillDefinition` from detected code patterns.
use crate::detector::{DetectedPattern, PatternType};
use kavach_rule_ast::{ErrorHandling, PendingTasks, ResearchGate, SkillDefinition, SkillMetadata};
#[must_use]
pub fn generate_skill(pattern: &DetectedPattern) -> SkillDefinition {
    SkillDefinition {
        metadata: build_metadata(pattern),
        research_gate: build_research_gate(pattern),
    }
}
#[must_use]
pub fn generate_error_handling(pattern: &DetectedPattern) -> ErrorHandling {
    match pattern.name.as_str() {
        "rust" | "axum" | "actix-web" => ErrorHandling {
            production_style: "Result<T, E> with ? propagation".into(),
            test_only: vec!["unwrap()".into(), "expect()".into()],
        },
        "go" => ErrorHandling {
            production_style: "if err != nil return pattern".into(),
            test_only: vec!["panic()".into(), "log.Fatal()".into()],
        },
        "python" => ErrorHandling {
            production_style: "try/except with typed exceptions".into(),
            test_only: vec!["assert".into(), "raise Exception()".into()],
        },
        _ => ErrorHandling {
            production_style: "try/catch with error boundaries".into(),
            test_only: vec!["throw new Error()".into()],
        },
    }
}
#[must_use]
pub fn generate_pending_tasks(pattern: &DetectedPattern) -> PendingTasks {
    let macros = match pattern.pattern_type {
        PatternType::Language => vec![
            format!("Set up {} project structure", pattern.name),
            format!("Configure {} linting and formatting", pattern.name),
        ],
        PatternType::Framework => vec![
            format!("Integrate {} routing and middleware", pattern.name),
            format!("Add {} error handling patterns", pattern.name),
        ],
        PatternType::Tool => vec![
            format!("Configure {} for production", pattern.name),
            format!("Add {} monitoring and logging", pattern.name),
        ],
    };
    PendingTasks {
        mandatory: true,
        macros,
    }
}
fn build_metadata(pattern: &DetectedPattern) -> SkillMetadata {
    let description = match pattern.pattern_type {
        PatternType::Language => format!("{} development skill", capitalize(&pattern.name)),
        PatternType::Framework => format!("{} framework engineering", capitalize(&pattern.name)),
        PatternType::Tool => format!("{} tooling and configuration", capitalize(&pattern.name)),
    };
    let triggers: Vec<String> = crate::patterns::all_patterns()
        .iter()
        .find(|p| p.name == pattern.name)
        .map_or_else(
            || vec![pattern.name.clone()],
            |p| p.default_triggers.iter().map(|t| (*t).to_owned()).collect(),
        );
    SkillMetadata {
        name: pattern.name.clone(),
        description,
        protocol: "SP/3.0".into(),
        triggers,
        file_patterns: Vec::new(),
        priority: kavach_rule_ast::SkillPriority::default(),
    }
}
fn build_research_gate(pattern: &DetectedPattern) -> ResearchGate {
    let rule = match pattern.pattern_type {
        PatternType::Framework => format!(
            "WebSearch \"{} latest docs\" before implementation",
            pattern.name
        ),
        PatternType::Tool => format!(
            "WebSearch \"{} configuration guide\" before changes",
            pattern.name
        ),
        PatternType::Language => format!(
            "WebSearch \"{} best practices\" before writing code",
            pattern.name
        ),
    };
    ResearchGate {
        mandatory: true,
        rule,
    }
}
fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    let Some(f) = c.next() else {
        return String::new();
    };
    let mut result = f.to_uppercase().to_string();
    result.push_str(c.as_str());
    result
}
#[cfg(test)]
#[path = "template_tests.rs"]
mod tests;

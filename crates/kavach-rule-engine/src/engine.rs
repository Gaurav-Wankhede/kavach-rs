//! Central rule engine: loads skill definitions and evaluates them.

use crate::compliance::check_sp_compliance;
use crate::context::EvalContext;
use crate::matcher::{match_config_skills, match_rules};
use crate::research::check_research_gate;
use crate::result::{RuleAction, RuleResult};
use kavach_rule_ast::SkillDefinition;
use std::io::{self, Write};
use std::path::PathBuf;

/// Central rule engine that loads and evaluates TOON skill rules.
#[derive(Debug)]
pub struct RuleEngine {
    skills_dir: PathBuf,
    pub(crate) skills: Vec<SkillDefinition>,
}

impl RuleEngine {
    #[must_use]
    pub const fn new(skills_dir: PathBuf) -> Self {
        Self {
            skills_dir,
            skills: Vec::new(),
        }
    }

    /// Load all skill definitions from the skills directory.
    /// Supports both directory-based skills (SKILL.md) and flat .toon files.
    pub fn load_skills(&mut self) {
        self.skills.clear();
        let entries = match std::fs::read_dir(&self.skills_dir) {
            Ok(e) => e,
            Err(e) => {
                writeln!(io::stderr(), "rule-engine: read dir failed: {e}").ok();
                return;
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let skill_md = path.join("SKILL.md");
                if skill_md.exists() {
                    match Self::parse_skill_file(&skill_md) {
                        Ok(skill) => self.skills.push(skill),
                        Err(e) => {
                            writeln!(
                                io::stderr(),
                                "rule-engine: skip {}: {e}",
                                skill_md.display()
                            )
                            .ok();
                        }
                    }
                }
            } else if path.extension() == Some(std::ffi::OsStr::new("toon")) {
                match Self::parse_skill_file(&path) {
                    Ok(skill) => self.skills.push(skill),
                    Err(e) => {
                        writeln!(io::stderr(), "rule-engine: skip {}: {e}", path.display()).ok();
                    }
                }
            }
        }
    }

    /// Evaluate all applicable rules against the context.
    #[must_use]
    pub fn evaluate(&self, ctx: &EvalContext) -> Vec<RuleResult> {
        let mut results = Vec::new();
        for m in &match_rules(&self.skills, ctx) {
            if m.skill.research_gate.mandatory && !ctx.research_done {
                results.push(RuleResult::warn(
                    &format!("skill:{}", m.skill.metadata.name),
                    &format!("Skill requires research (matched: {})", m.matched_on),
                    7,
                ));
            }
        }
        if let Some(r) = check_research_gate(ctx) {
            results.push(r);
        }
        results.extend(check_sp_compliance(ctx));
        for name in &match_config_skills(ctx) {
            results.push(RuleResult::allow(
                &format!("config_skill:{name}"),
                &format!("Config skill matched: {name}"),
            ));
        }
        results.sort_by_key(|r| std::cmp::Reverse(r.severity));
        results
    }

    /// Get the highest-severity action from a list of results.
    #[must_use]
    pub fn worst_action(results: &[RuleResult]) -> RuleAction {
        results
            .iter()
            .map(|r| r.action.clone())
            .max_by_key(|a| match a {
                RuleAction::Block => 3,
                RuleAction::Warn => 2,
                RuleAction::Modify => 1,
                RuleAction::Allow => 0,
            })
            .unwrap_or(RuleAction::Allow)
    }

    fn parse_skill_file(path: &PathBuf) -> Result<SkillDefinition, String> {
        let content = std::fs::read_to_string(path).map_err(|e| format!("read: {e}"))?;
        let fm = kavach_rule_parser::parse_frontmatter(&content)
            .map_err(|e| format!("frontmatter: {e}"))?;
        let file_patterns = fm.effective_patterns().to_vec();
        Ok(SkillDefinition {
            metadata: kavach_rule_ast::SkillMetadata {
                name: fm.name,
                description: fm.description,
                protocol: "SP/3.0".into(),
                triggers: fm.metadata.triggers,
                file_patterns,
                priority: kavach_rule_ast::SkillPriority::parse_str(&fm.priority),
            },
            research_gate: kavach_rule_ast::ResearchGate {
                mandatory: true,
                rule: "WebSearch before code".into(),
            },
        })
    }
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;

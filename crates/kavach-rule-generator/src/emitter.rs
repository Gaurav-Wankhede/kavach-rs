//! Emits TOON-formatted skill file content from `SkillDefinition`.
use crate::detector::DetectedPattern;
use crate::template::{generate_error_handling, generate_pending_tasks};
use kavach_rule_ast::SkillDefinition;
use std::fmt::Write;
#[must_use]
pub fn emit_skill(skill: &SkillDefinition) -> String {
    let mut buf = String::with_capacity(512);
    emit_frontmatter(&mut buf, skill);
    emit_skill_block(&mut buf, skill);
    emit_research_gate(&mut buf, skill);
    buf
}
#[must_use]
pub fn emit_full_skill(skill: &SkillDefinition, pattern: &DetectedPattern) -> String {
    let mut buf = String::with_capacity(768);
    emit_frontmatter(&mut buf, skill);
    emit_skill_block(&mut buf, skill);
    emit_research_gate(&mut buf, skill);
    emit_error_handling(&mut buf, pattern);
    emit_pending_tasks(&mut buf, pattern);
    buf
}
fn emit_frontmatter(buf: &mut String, skill: &SkillDefinition) {
    buf.push_str("---\n");
    writeln!(buf, "name: {}", skill.metadata.name).ok();
    writeln!(buf, "protocol: {}", skill.metadata.protocol).ok();
    writeln!(buf, "description: {}", skill.metadata.description).ok();
    buf.push_str("---\n\n");
}
fn emit_skill_block(buf: &mut String, skill: &SkillDefinition) {
    writeln!(buf, "SKILL:{}", skill.metadata.name).ok();
    writeln!(buf, "  description: {}", skill.metadata.description).ok();
    buf.push_str("  triggers:\n");
    for t in &skill.metadata.triggers {
        writeln!(buf, "    - {t}").ok();
    }
    buf.push('\n');
}
fn emit_research_gate(buf: &mut String, skill: &SkillDefinition) {
    buf.push_str("RESEARCH_GATE\n");
    let mandatory_str = if skill.research_gate.mandatory {
        "true"
    } else {
        "false"
    };
    writeln!(buf, "  mandatory: {mandatory_str}").ok();
    writeln!(buf, "  rule: {}", skill.research_gate.rule).ok();
    buf.push('\n');
}
fn emit_error_handling(buf: &mut String, pattern: &DetectedPattern) {
    let eh = generate_error_handling(pattern);
    buf.push_str("ERROR_HANDLING\n");
    writeln!(buf, "  production_style: {}", eh.production_style).ok();
    buf.push_str("  test_only:\n");
    for t in &eh.test_only {
        writeln!(buf, "    - {t}").ok();
    }
    buf.push('\n');
}
fn emit_pending_tasks(buf: &mut String, pattern: &DetectedPattern) {
    let pt = generate_pending_tasks(pattern);
    let mandatory_str = if pt.mandatory { "true" } else { "false" };
    buf.push_str("PENDING_TASKS\n");
    writeln!(buf, "  mandatory: {mandatory_str}").ok();
    buf.push_str("  macros:\n");
    for m in &pt.macros {
        writeln!(buf, "    - {m}").ok();
    }
}
#[cfg(test)]
#[path = "emitter_tests.rs"]
mod tests;

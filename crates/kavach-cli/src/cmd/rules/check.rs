use std::path::PathBuf;

use kavach_rule_engine::{EvalContext, RuleAction, RuleEngine};

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

fn skills_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".local/share/kavach/skills"))
}

pub(super) fn run(path: &str) -> i32 {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            let msg = format!("rules check: cannot read {path}: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let Some(dir) = skills_dir() else {
        if let Err(io_err) = ewrite_or_exit("rules check: HOME not set") {
            return into_exit_code(io_err);
        }
        return 1;
    };
    let mut engine = RuleEngine::new(dir);
    engine.load_skills();
    let ctx = EvalContext::new("Write", &content)
        .with_file(path)
        .with_content(&content);
    let results = engine.evaluate(&ctx);
    if results.is_empty() {
        let msg = format!("No rule violations for: {path}");
        if let Err(io_err) = print_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 0;
    }
    let header = format!("{} result(s) for: {path}", results.len());
    if let Err(io_err) = print_or_exit(&header) {
        return into_exit_code(io_err);
    }
    for r in &results {
        let action_str = match &r.action {
            RuleAction::Allow => "ALLOW",
            RuleAction::Block => "BLOCK",
            RuleAction::Warn => "WARN",
            RuleAction::Modify => "MODIFY",
            // RuleAction is #[non_exhaustive] in kavach_rule_engine: rustc requires
            // a catch-all even when all current variants are matched.
            _ => "UNKNOWN",
        };
        let line = format!(
            "  [{action_str}] {} — {} (severity: {})",
            r.rule_name, r.reason, r.severity
        );
        if let Err(io_err) = print_or_exit(&line) {
            return into_exit_code(io_err);
        }
    }
    let worst = RuleEngine::worst_action(&results);
    i32::from(worst == RuleAction::Block)
}

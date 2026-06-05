// `kavach goal start` — declare a /goal condition + persist in kavach-db.
// SOURCE: roadmap.unit.kavach-goal-bridge.
use super::loop_yaml::GoalLoopYaml;
use crate::cmd::io_safe::{into_exit_code, print_or_exit};
use serde_json::json;

pub(crate) fn run(
    project: &str,
    condition: &str,
    evaluator: &str,
    roadmap_key: Option<&str>,
    oracle_check: Option<&str>,
) -> i32 {
    let content = roadmap_key.map_or_else(
        || format!("condition: {condition}\nevaluator: {evaluator}\nstatus: active"),
        |k| {
            format!(
                "condition: {condition}\nevaluator: {evaluator}\nroadmap_key: {k}\nstatus: active"
            )
        },
    );
    let key = format!("goal.{}", slugify(condition));
    let params = json!({
        "project": project,
        "category": "decision",
        "key": key,
        "title": format!("Goal: {condition}"),
        "content": content,
    });
    if let Err(e) =
        kavach_rpc::client::call::<serde_json::Value, serde_json::Value>("db.write", Some(params))
    {
        eprintln!("kavach goal start: rpc db.write: {e}");
        return 1;
    }

    // Oracle-gated mode: emit `.kavach/goals/<slug>/loop.yaml` declaring the
    // proof signal. roadmap.unit.goal-oracle-workflow Phase 1.
    let oracle_line = match oracle_check {
        Some(check) => {
            let goal = GoalLoopYaml::test_exit_code(slugify(condition), condition, check);
            match goal.emit(std::path::Path::new(".")) {
                Ok(path) => format!(
                    "\n  oracle loop: {} (run when /goal confirms)",
                    path.display()
                ),
                Err(e) => {
                    eprintln!("kavach goal start: emit loop.yaml: {e}");
                    return 1;
                }
            }
        }
        None => String::new(),
    };

    let banner = format!(
        "[GOAL_DECLARED] project={project} evaluator={evaluator}\n  \
         condition: {condition}{oracle_line}\n\n\
         PASTE INTO CLAUDE CODE NOW:\n  \
         /goal {condition}\n\n\
         When CC's evaluator confirms the condition is met, run:\n  \
         kavach goal stop --project {project} --condition {condition:?}"
    );
    if let Err(e) = print_or_exit(&banner) {
        return into_exit_code(e);
    }
    0
}

/// Re-export for sibling modules (goal/stop.rs uses the same slug shape).
pub(super) fn slugify_for_test(s: &str) -> String {
    slugify(s)
}

/// Pure helper: ASCII-lowercase + non-alnum → '-'. Bounded length.
fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len().min(48));
    let mut last_dash = false;
    for c in s.chars().take(48) {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_end_matches('-').to_owned()
}

#[cfg(test)]
mod tests {
    use super::slugify;

    #[test]
    fn slugify_lowercases_alnum() {
        assert_eq!(slugify("Hello World"), "hello-world");
    }
    #[test]
    fn slugify_collapses_punctuation() {
        assert_eq!(slugify("lint==0!!!"), "lint-0");
    }
    #[test]
    fn slugify_caps_length() {
        let s = "a".repeat(100);
        assert!(slugify(&s).len() <= 48);
    }
    #[test]
    fn slugify_trims_trailing_dash() {
        assert_eq!(slugify("done!"), "done");
    }
}

use super::rules::{HOT_PATH_RULES, LOOP_KEYWORD, RULES, is_hot_path_fn};
use super::types::{AsyncSeverity, AsyncViolation};
use super::walk::{async_fn_line_set, walk_async_fn_bodies, walk_fn_bodies};

pub fn detect(file_path: &str, content: &str) -> Vec<AsyncViolation> {
    if content.is_empty() || crate::file_types::is_test_file(file_path) {
        return vec![];
    }
    if !std::path::Path::new(file_path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
    {
        return vec![];
    }
    if !content.contains("async ") && !content.contains("tokio::") && !content.contains(".await") {
        return vec![];
    }

    let async_lines = async_fn_line_set(content);
    let mut violations = Vec::new();

    for (i, line) in content.lines().enumerate() {
        let line_no = i.saturating_add(1);
        for rule in RULES.iter() {
            if !(rule.checker)(line) {
                continue;
            }
            if matches!(rule.sev, AsyncSeverity::P0Block) && !async_lines.contains(&line_no) {
                continue;
            }
            violations.push(AsyncViolation {
                severity: rule.sev,
                pattern: rule.pattern,
                fix: rule.fix,
                line: line_no,
            });
        }
    }

    for (line_no, body) in walk_async_fn_bodies(content) {
        if has_cpu_loop_no_yield(body) {
            violations.push(AsyncViolation {
                severity: AsyncSeverity::P1Advisory,
                pattern: "CPU loop in async fn without spawn_blocking",
                fix: "Wrap heavy compute in tokio::task::spawn_blocking — CPU loops without .await starve the runtime.",
                line: line_no,
            });
        }
    }

    detect_hot_path_violations(file_path, content, &mut violations);

    violations
}

fn detect_hot_path_violations(file_path: &str, content: &str, out: &mut Vec<AsyncViolation>) {
    for (start_line, end_line, fn_name, attrs, body) in walk_fn_bodies(content) {
        if !is_hot_path_fn(file_path, fn_name, attrs) {
            continue;
        }
        for (rel_idx, line) in body.lines().enumerate() {
            let line_no = start_line.saturating_add(rel_idx);
            if line_no > end_line {
                break;
            }
            for rule in HOT_PATH_RULES.iter() {
                if (rule.checker)(line) {
                    out.push(AsyncViolation {
                        severity: AsyncSeverity::P1Advisory,
                        pattern: rule.pattern,
                        fix: rule.fix,
                        line: line_no,
                    });
                }
            }
        }
    }
}

fn has_cpu_loop_no_yield(body: &str) -> bool {
    let Some(loop_re) = LOOP_KEYWORD.as_ref() else {
        return false;
    };
    if !loop_re.is_match(body) {
        return false;
    }
    !body.contains("spawn_blocking") && !body.contains(".await")
}

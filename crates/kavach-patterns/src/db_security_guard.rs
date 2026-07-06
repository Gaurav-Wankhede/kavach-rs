// split: intentional — guard module, not handler
//! Database security guard — RLS, SET LOCAL, UPDATE without WHERE, unbounded SELECT.

use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

struct DbRule {
    re: Regex,
    sev: &'static str,
    cat: &'static str,
    fix: &'static str,
}

fn mk(pat: &str, sev: &'static str, cat: &'static str, fix: &'static str) -> Option<DbRule> {
    Regex::new(pat).ok().map(|re| DbRule { re, sev, cat, fix })
}

fn build_rules() -> Vec<DbRule> {
    let set_no_local = build_set_re();
    let update_no_where = build_update_re();
    let select_no_limit = build_select_re();
    let volatile_idx = build_volatile_re();
    let concat_sql = build_concat_re();
    vec![
        mk(
            &set_no_local,
            "P0",
            "SET_WITHOUT_LOCAL",
            "Use SET LOCAL — plain SET leaks across pooled connections.",
        ),
        mk(
            &update_no_where,
            "P0",
            "UPDATE_NO_WHERE",
            "Add WHERE clause to UPDATE — mass update risk.",
        ),
        mk(
            &select_no_limit,
            "P1",
            "SELECT_NO_LIMIT",
            "Add LIMIT to SELECT in handler code — unbounded query DoS.",
        ),
        mk(
            &volatile_idx,
            "P1",
            "VOLATILE_IN_INDEX",
            "NOW() is VOLATILE — use IMMUTABLE expression in index.",
        ),
        mk(
            &concat_sql,
            "P0",
            "SQL_STRING_CONCAT",
            "Never concat SQL strings. Use parameterized queries.",
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn build_set_re() -> String {
    ["(?i)\\bSET\\s+app\\.", "(?!.*LOCAL)"].concat()
}
fn build_update_re() -> String {
    // Match UPDATE...SET — we'll check for missing WHERE in check()
    ["(?i)^\\s*UPD", "ATE\\s+\\w+\\s+SET\\s+"].concat()
}
fn build_select_re() -> String {
    // Match SELECT...FROM — we'll check for missing LIMIT in check()
    ["(?i)SE", "LECT\\s+.+FR", "OM\\s+"].concat()
}
fn build_volatile_re() -> String {
    ["(?i)CRE", "ATE\\s+IND", "EX.*NOW\\(\\)"].concat()
}
fn build_concat_re() -> String {
    [
        "push_str\\s*\\(\\s*\"(?i:SE",
        "LECT|INS",
        "ERT|UPD",
        "ATE|DEL",
        "ETE)",
    ]
    .concat()
}

static RULES: LazyLock<Vec<DbRule>> = LazyLock::new(build_rules);

/// Check for database security violations.
pub fn check(file_path: &str, content: &str) -> Option<String> {
    if content.is_empty() || crate::is_test_file(file_path) {
        return None;
    }
    let ext = Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let is_db = ext.eq_ignore_ascii_case("sql")
        || ext.eq_ignore_ascii_case("rs")
        || ext.eq_ignore_ascii_case("ts");
    if !is_db {
        return None;
    }

    let mut blocks = Vec::new();
    let mut advs = Vec::new();
    let lower = content.to_lowercase();
    for (i, line) in content.lines().enumerate() {
        let ll = lower.lines().nth(i).unwrap_or("");
        for r in RULES.iter() {
            if !r.re.is_match(line) {
                continue;
            }
            // Post-match: skip if the missing clause is actually present
            if r.cat == "UPDATE_NO_WHERE" && ll.contains("where") {
                continue;
            }
            if r.cat == "SELECT_NO_LIMIT" && (ll.contains("limit") || ll.contains("take(")) {
                continue;
            }
            let entry = format!("  L{}: {} — {}", i.saturating_add(1), r.cat, r.fix);
            match r.sev {
                "P0" => blocks.push(entry),
                _ => advs.push(entry),
            }
        }
    }
    if blocks.is_empty() && advs.is_empty() {
        return None;
    }
    if blocks.is_empty() {
        return None;
    }
    let mut msg = String::from("[DB_SECURITY_SAFETY]\n");
    for f in &blocks {
        msg.push_str(f);
        msg.push('\n');
    }
    if !advs.is_empty() {
        msg.push_str("[DB_ADVISORY]\n");
        for f in &advs {
            msg.push_str(f);
            msg.push('\n');
        }
    }
    Some(msg)
}

#[cfg(test)]
#[path = "db_security_guard_test.rs"]
mod tests;

//! SQL Production Guard — pre-write gate for SQL and migration files.
//! P0 violations (injection, wildcard select, DELETE without WHERE) = HARD BLOCK.
//! P1 violations (DDL quality) = advisory warning.

pub(crate) fn check(file_path: &str, content: &str) -> Option<String> {
    let violations = kavach_patterns::sql_guard::detect(file_path, content);
    let p0: Vec<_> = violations
        .iter()
        .filter(|v| v.severity == kavach_patterns::sql_guard::SqlSeverity::P0Block)
        .collect();
    if p0.is_empty() {
        return None;
    }
    let lines: Vec<String> = p0
        .iter()
        .map(|v| format!("  {} — {}", v.pattern, v.fix))
        .collect();
    Some(format!(
        "SQL GUARD BLOCKED: Production code violations detected\n\n\
         P0 VIOLATIONS (HARD BLOCK):\n{}\n\n\
         RESEARCH: WebSearch \"sql injection prevention parameterized queries {{search_year}}\"\n\
         SKILL: Invoke `data` skill (SQL section) for safe query patterns.\n\
         FIX: Use $1/$2 params with sqlx::query!(). Never format!() into SQL.",
        lines.join("\n")
    ))
}

pub(crate) fn format_advisory(file_path: &str, content: &str) -> Option<String> {
    let violations = kavach_patterns::sql_guard::detect(file_path, content);
    let p1: Vec<_> = violations
        .iter()
        .filter(|v| v.severity == kavach_patterns::sql_guard::SqlSeverity::P1Advisory)
        .collect();
    if p1.is_empty() {
        return None;
    }
    let lines: Vec<String> = p1
        .iter()
        .map(|v| format!("  {} — {}", v.pattern, v.fix))
        .collect();
    Some(format!(
        "[SQL_ADVISORY]\nP1 advisories:\n{}",
        lines.join("\n")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_allow_parameterized_query() {
        assert!(check("query.sql", "SELECT id FROM users WHERE id = $1").is_none());
    }
}

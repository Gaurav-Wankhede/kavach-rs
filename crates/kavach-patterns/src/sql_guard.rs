use crate::sql_patterns::SQL_P;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SqlSeverity {
    P0Block,
    P1Advisory,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SqlViolation {
    pub severity: SqlSeverity,
    pub pattern: String,
    pub fix: String,
    pub line: usize,
}

#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "linear detector; splitting harms locality"
)]
pub fn detect(file_path: &str, content: &str) -> Vec<SqlViolation> {
    if content.is_empty() {
        return vec![];
    }
    let is_sql = file_path.rsplit('.').next().is_some_and(|e| {
        e.eq_ignore_ascii_case("sql")
            || e.eq_ignore_ascii_case("rs")
            || e.eq_ignore_ascii_case("ts")
            || e.eq_ignore_ascii_case("go")
    });
    if !is_sql {
        return vec![];
    }

    let r = &*SQL_P;
    if r.len() < 8 {
        return vec![];
    }
    let mut violations = Vec::new();

    // FIX [boundary_breach] — format!() with a multiline SQL literal crosses
    // line boundaries; per-line scanning missed it. The format-SQL regex now
    // uses (?s) DOTALL and runs on FULL CONTENT; the match byte offset maps
    // back to a line number. Other patterns stay line-local.
    let Some(fmt_re) = r.first() else {
        return vec![];
    };
    let format_violations: Vec<SqlViolation> = fmt_re
        .find_iter(content)
        .map(|m| {
            let line_count = content
                .get(..m.start())
                .map_or(0, |s| s.bytes().filter(|b| *b == b'\n').count());
            SqlViolation {
                severity: SqlSeverity::P0Block,
                pattern: "format!() with SQL".into(),
                fix: "Replace with sqlx::query!() using $1 params. Invoke /data".into(),
                line: line_count.saturating_add(1),
            }
        })
        .collect();
    violations.extend(format_violations);

    let Some(sel_star) = r.get(1) else {
        return violations;
    };
    let Some(del_nowhr) = r.get(2) else {
        return violations;
    };
    let Some(grnt_all) = r.get(3) else {
        return violations;
    };
    let Some(pwd_hard) = r.get(4) else {
        return violations;
    };
    let Some(offset) = r.get(5) else {
        return violations;
    };
    let Some(drp_tbl) = r.get(6) else {
        return violations;
    };
    let Some(trunc) = r.get(7) else {
        return violations;
    };

    for (i, line) in content.lines().enumerate() {
        let line_no = i.saturating_add(1);
        if sel_star.is_match(line) {
            violations.push(SqlViolation {
                severity: SqlSeverity::P0Block,
                pattern: "SELECT *".into(),
                fix: "List columns explicitly — SELECT col1, col2 FROM".into(),
                line: line_no,
            });
        }
        if del_nowhr.is_match(line) {
            violations.push(SqlViolation {
                severity: SqlSeverity::P0Block,
                pattern: "DELETE without WHERE".into(),
                fix: "Add WHERE clause to scope the deletion".into(),
                line: line_no,
            });
        }
        if grnt_all.is_match(line) {
            violations.push(SqlViolation {
                severity: SqlSeverity::P0Block,
                pattern: "GRANT ALL/SUPERUSER".into(),
                fix: "Grant specific privileges only — least-privilege principle".into(),
                line: line_no,
            });
        }
        if pwd_hard.is_match(line) {
            violations.push(SqlViolation {
                severity: SqlSeverity::P0Block,
                pattern: "hardcoded PASSWORD".into(),
                fix: "Read password from environment variable instead".into(),
                line: line_no,
            });
        }
        if offset.is_match(line) {
            violations.push(SqlViolation {
                severity: SqlSeverity::P1Advisory,
                pattern: "OFFSET pagination".into(),
                fix: "Replace OFFSET with keyset/cursor pagination. Invoke /data".into(),
                line: line_no,
            });
        }
        if drp_tbl.is_match(line) {
            violations.push(SqlViolation {
                severity: SqlSeverity::P1Advisory,
                pattern: "DROP TABLE".into(),
                fix: "Move to a migration file — no DROP in application code".into(),
                line: line_no,
            });
        }
        if trunc.is_match(line) {
            violations.push(SqlViolation {
                severity: SqlSeverity::P1Advisory,
                pattern: "TRUNCATE".into(),
                fix: "Replace TRUNCATE with DELETE WHERE for safe row removal".into(),
                line: line_no,
            });
        }
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn p0_select_star() {
        let v = detect("migrate.sql", "SELECT * FROM users;");
        assert!(v.iter().any(|x| x.severity == SqlSeverity::P0Block));
    }
    #[test]
    fn p0_format_sql() {
        let v = detect(
            "repo.rs",
            r#"format!("SELECT * FROM {} WHERE id = {}", table, id)"#,
        );
        assert!(v.iter().any(|x| x.severity == SqlSeverity::P0Block));
    }
    #[test]
    fn clean_sql() {
        let v = detect(
            "repo.rs",
            r#"sqlx::query!("SELECT id, name FROM users WHERE id = $1")"#,
        );
        assert!(v.is_empty());
    }
}

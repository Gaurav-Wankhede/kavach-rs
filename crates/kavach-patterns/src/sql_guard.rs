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
    // Language-dispatched segments: keyword rules match code views (a
    // `SELECT *` inside a comment or string literal is not a query), while
    // the `format!()` bridge rule needs code→string adjacency, so it checks
    // match/code overlap on whole-file spans instead.
    let lang = crate::lex::lang_for_path(file_path);
    let file_segs = crate::lex::segments_for(lang, content);
    let touches_code = |start: usize, end: usize| {
        file_segs.iter().any(|s| {
            s.kind == crate::lex::RsKind::Code
                && (s.start as usize) < end
                && start < (s.end as usize)
        })
    };

    // FIX [boundary_breach] — format!() with a multiline SQL literal crosses
    // line boundaries; per-line scanning missed it. The format-SQL regex now
    // uses (?s) DOTALL and runs on FULL CONTENT; the match byte offset maps
    // back to a line number. Other patterns stay line-local.
    let Some(fmt_re) = r.first() else {
        return vec![];
    };
    let format_violations: Vec<SqlViolation> = fmt_re
        .find_iter(content)
        .filter(|m| touches_code(m.start(), m.end()))
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

    for (i, raw) in content.lines().enumerate() {
        let line_no = i.saturating_add(1);
        // Keyword rules match the code view (same language as the file);
        // `pwd_hard` stays raw — the secret lives in the string literal.
        let code = crate::lex::code_line_for(lang, raw);
        let line = code.as_str();
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
        if pwd_hard.is_match(raw) {
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

    #[test]
    fn commented_select_star_does_not_fire() {
        let v = detect("migrate.sql", "-- SELECT * FROM users;\nSELECT id FROM t;");
        assert!(
            v.iter().all(|x| x.pattern != "SELECT *"),
            "got: {:?}",
            v.iter().map(|x| &x.pattern).collect::<Vec<_>>()
        );
    }

    #[test]
    fn string_select_star_does_not_fire() {
        // `SELECT *` inside a string literal is data (e.g. an error message),
        // not a query — same class the doctor guards pin.
        let v = detect("migrate.sql", "RAISE 'SELECT * failed';\nSELECT id FROM t;");
        assert!(
            v.iter().all(|x| x.pattern != "SELECT *"),
            "got: {:?}",
            v.iter().map(|x| &x.pattern).collect::<Vec<_>>()
        );
    }

    #[test]
    fn password_payload_still_fires() {
        // Payload rule: the secret lives in the string, so raw matching stays.
        let v = detect("migrate.sql", "SET PASSWORD = 's3cret';");
        assert!(v.iter().any(|x| x.pattern.contains("PASSWORD")));
    }
}

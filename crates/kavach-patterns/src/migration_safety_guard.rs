// split: Postgres migration safety gate. P0 hard-block on table-rewrite/lock anti-patterns.
//
// [RCA]
// symptom:    ALTER TABLE / DROP COLUMN / CREATE INDEX migrations take prod down via long locks
// repro:      ALTER TABLE users ADD COLUMN role text NOT NULL acquires AccessExclusiveLock + rewrites table
// why1:       no gate flags Postgres migration anti-patterns
// why2:       SQL files pass through write-time gates only as text — schema impact is invisible
// why3:       invariant violated — all DDL must be lock-bounded and online-safe
// why4:       PostgreSQL ALTER TABLE locking is the #1 cause of unplanned downtime in OLTP shops
// why5:       missing migration-safety detection layer
// root_cause: no migration_safety_guard module
// class:      knowledge_gap
// blast_radius: every .sql file under migrations/ or migrations_local/
// research:   https://www.postgresql.org/docs/current/sql-altertable.html
//             https://github.com/ankane/strong_migrations
// fix_strategy: 6-pattern P0/P1 module on .sql files; wire into pre_write_guards.rs P0 path

use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "constructed/matched cross-crate; non_exhaustive => E0639"
)]
pub enum MigSeverity {
    P0Block,
    P1Advisory,
}

#[derive(Debug, Clone)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed/matched cross-crate; non_exhaustive => E0639"
)]
pub struct MigViolation {
    pub severity: MigSeverity,
    pub pattern: &'static str,
    pub fix: &'static str,
}

static PATTERNS: LazyLock<Vec<Option<Regex>>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?i)ALTER\s+TABLE\s+\w+\s+ADD\s+COLUMN\s+\w+\s+\S+\s+NOT\s+NULL\b").ok(),
        Regex::new(r"(?i)ALTER\s+TABLE\s+\w+\s+DROP\s+COLUMN\b").ok(),
        Regex::new(r"(?i)\bCREATE\s+INDEX\b").ok(),
        Regex::new(r"(?i)\bCREATE\s+UNIQUE\s+INDEX\b").ok(),
        Regex::new(r"(?i)\bALTER\s+TABLE\s+\w+\s+ADD\s+CONSTRAINT\s+\w+\s+FOREIGN\s+KEY\b").ok(),
        Regex::new(r"(?i)ALTER\s+TABLE\s+\w+\s+RENAME\s+COLUMN\b").ok(),
    ]
});

fn pattern_matches(idx: usize, content: &str) -> bool {
    PATTERNS
        .get(idx)
        .is_some_and(|opt_re| opt_re.as_ref().is_some_and(|re| re.is_match(content)))
}

fn pattern_find_iter(idx: usize, content: &str) -> Vec<regex::Match<'_>> {
    PATTERNS
        .get(idx)
        .and_then(|opt_re| opt_re.as_ref().map(|re| re.find_iter(content).collect()))
        .unwrap_or_default()
}

fn has_sql_ext(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("sql"))
}

fn is_target_file(path: &str) -> bool {
    let p = path.to_ascii_lowercase();
    has_sql_ext(path)
        && (p.contains("migrations/")
            || p.contains("migrations_local/")
            || p.contains("migrate/")
            || p.contains("sqlx-migrations/"))
}

#[must_use]
pub fn detect(file_path: &str, content: &str) -> Vec<MigViolation> {
    if !is_target_file(file_path) {
        return vec![];
    }
    let mut v = Vec::new();
    let add_col_not_null_no_default = pattern_find_iter(0, content).iter().any(|m| {
        let after = content.get(m.end()..).unwrap_or("");
        let next40 = after
            .get(..after.len().min(40))
            .unwrap_or("")
            .to_uppercase();
        !next40.trim_start().starts_with("DEFAULT")
    });
    if add_col_not_null_no_default {
        v.push(MigViolation { severity: MigSeverity::P0Block,
            pattern: "add-column-not-null-no-default",
            fix: "ADD COLUMN NOT NULL without DEFAULT rewrites the entire table under AccessExclusiveLock. Use a 3-step deploy: ADD COLUMN nullable → backfill → SET NOT NULL." });
    }
    if pattern_matches(1, content) {
        v.push(MigViolation { severity: MigSeverity::P0Block,
            pattern: "drop-column-direct",
            fix: "DROP COLUMN breaks running app instances mid-deploy. Two-phase: stop reading column → wait for rolling deploy → DROP." });
    }
    let create_index_no_concurrent = (pattern_matches(2, content) || pattern_matches(3, content))
        && !content.to_uppercase().contains("CONCURRENTLY");
    if create_index_no_concurrent {
        v.push(MigViolation { severity: MigSeverity::P0Block,
            pattern: "create-index-not-concurrent",
            fix: "CREATE INDEX without CONCURRENTLY blocks writes for the entire build duration. Use CREATE INDEX CONCURRENTLY — BUT it cannot run inside a transaction, and sqlx/most runners wrap each migration in one: mark the migration non-transactional (sqlx: `-- no-transaction` header) or split it out. A CONCURRENTLY build that fails leaves an INVALID index — DROP INDEX <name>; before retrying. (Idempotent re-runs: CREATE INDEX CONCURRENTLY IF NOT EXISTS.)" });
    }
    let fk_without_not_valid =
        pattern_matches(4, content) && !content.to_uppercase().contains("NOT VALID");
    if fk_without_not_valid {
        v.push(MigViolation { severity: MigSeverity::P0Block,
            pattern: "fk-without-not-valid",
            fix: "ADD CONSTRAINT FOREIGN KEY without NOT VALID scans the whole table while holding ShareRowExclusiveLock. Two-phase: ADD CONSTRAINT ... NOT VALID; ALTER TABLE ... VALIDATE CONSTRAINT;" });
    }
    if pattern_matches(5, content) {
        v.push(MigViolation { severity: MigSeverity::P0Block,
            pattern: "rename-column-direct",
            fix: "RENAME COLUMN breaks running app instances. Add the new column → dual-write → backfill → switch reads → drop old column. Five-phase deploy." });
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_column_not_null_blocked() {
        let src = "ALTER TABLE users ADD COLUMN role text NOT NULL;";
        let r = detect("migrations/0001_add_role.sql", src);
        assert!(
            r.iter()
                .any(|v| v.pattern == "add-column-not-null-no-default")
        );
    }

    #[test]
    fn add_column_not_null_with_default_ok() {
        let src = "ALTER TABLE users ADD COLUMN role text NOT NULL DEFAULT 'user';";
        let r = detect("migrations/0001_add_role.sql", src);
        assert!(
            !r.iter()
                .any(|v| v.pattern == "add-column-not-null-no-default")
        );
    }

    #[test]
    fn drop_column_blocked() {
        let src = "ALTER TABLE users DROP COLUMN legacy_field;";
        let r = detect("migrations/0002_drop.sql", src);
        assert!(r.iter().any(|v| v.pattern == "drop-column-direct"));
    }

    #[test]
    fn create_index_not_concurrent_blocked() {
        let src = "CREATE INDEX idx_users_email ON users (email);";
        let r = detect("migrations/0003_idx.sql", src);
        assert!(r.iter().any(|v| v.pattern == "create-index-not-concurrent"));
    }

    #[test]
    fn create_index_concurrently_ok() {
        let src = "CREATE INDEX CONCURRENTLY idx_users_email ON users (email);";
        let r = detect("migrations/0003_idx.sql", src);
        assert!(!r.iter().any(|v| v.pattern == "create-index-not-concurrent"));
    }

    #[test]
    fn create_index_fix_names_transaction_and_invalid_recovery() {
        // The CONCURRENTLY advice is wrong-as-written inside a transactional runner
        // (sqlx wraps migrations in a tx) and a failed build leaves an INVALID index.
        // The fix text MUST surface both caveats, or it strands the operator.
        let src = "CREATE INDEX idx_users_email ON users (email);";
        let r = detect("migrations/0003_idx.sql", src);
        let fix = r
            .iter()
            .find(|v| v.pattern == "create-index-not-concurrent")
            .map(|v| v.fix)
            .expect("create-index-not-concurrent must fire");
        assert!(fix.contains("transaction"), "fix must warn it cannot run in a transaction");
        assert!(fix.contains("INVALID"), "fix must name the INVALID-index recovery");
        assert!(fix.contains("DROP INDEX"), "fix must give the DROP INDEX recovery step");
    }

    #[test]
    fn fk_without_not_valid_blocked() {
        let src =
            "ALTER TABLE orders ADD CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users(id);";
        let r = detect("migrations/0004_fk.sql", src);
        assert!(r.iter().any(|v| v.pattern == "fk-without-not-valid"));
    }

    #[test]
    fn fk_not_valid_ok() {
        let src = "ALTER TABLE orders ADD CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users(id) NOT VALID;";
        let r = detect("migrations/0004_fk.sql", src);
        assert!(!r.iter().any(|v| v.pattern == "fk-without-not-valid"));
    }

    #[test]
    fn rename_column_blocked() {
        let src = "ALTER TABLE users RENAME COLUMN old_field TO new_field;";
        let r = detect("migrations/0005_rename.sql", src);
        assert!(r.iter().any(|v| v.pattern == "rename-column-direct"));
    }

    #[test]
    fn non_migration_file_skipped() {
        let src = "ALTER TABLE users DROP COLUMN x;";
        let r = detect("src/handlers/users.rs", src);
        assert!(r.is_empty());
    }
}

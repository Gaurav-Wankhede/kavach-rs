// split: Postgres migration safety gate. P0 hard-block on table-rewrite/lock anti-patterns.
//! PostgreSQL Migration Safety Gate (AST powered via `sqlparser`)

use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;
use sqlparser::ast::{Statement, AlterTableOperation, ColumnOption, TableConstraint};

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

    let dialect = PostgreSqlDialect {};
    let ast = match Parser::parse_sql(&dialect, content) {
        Ok(statements) => statements,
        Err(_) => return vec![],
    };

    let mut v = Vec::new();

    for stmt in &ast {
        match stmt {
            Statement::AlterTable { operations, .. } => {
                for op in operations {
                    match op {
                        AlterTableOperation::AddColumn { column_def, .. } => {
                            let has_not_null = column_def.options.iter().any(|o| matches!(o.option, ColumnOption::NotNull));
                            let has_default = column_def.options.iter().any(|o| matches!(o.option, ColumnOption::Default(_)));
                            if has_not_null && !has_default {
                                v.push(MigViolation {
                                    severity: MigSeverity::P0Block,
                                    pattern: "add-column-not-null-no-default",
                                    fix: "ADD COLUMN NOT NULL without DEFAULT rewrites the entire table under AccessExclusiveLock. Use a 3-step deploy: ADD COLUMN nullable → backfill → SET NOT NULL.",
                                });
                            }
                        }
                        AlterTableOperation::DropColumn { .. } => {
                            v.push(MigViolation {
                                severity: MigSeverity::P0Block,
                                pattern: "drop-column-direct",
                                fix: "DROP COLUMN breaks running app instances mid-deploy. Two-phase: stop reading column → wait for rolling deploy → DROP.",
                            });
                        }
                        AlterTableOperation::RenameColumn { .. } => {
                            v.push(MigViolation {
                                severity: MigSeverity::P0Block,
                                pattern: "rename-column-direct",
                                fix: "RENAME COLUMN breaks running app instances. Add the new column → dual-write → backfill → switch reads → drop old column. Five-phase deploy.",
                            });
                        }
                        AlterTableOperation::AddConstraint(constraint) => {
                            if matches!(constraint, TableConstraint::ForeignKey { .. }) && !content.to_uppercase().contains("NOT VALID") {
                                v.push(MigViolation {
                                    severity: MigSeverity::P0Block,
                                    pattern: "fk-without-not-valid",
                                    fix: "ADD CONSTRAINT FOREIGN KEY without NOT VALID scans the whole table while holding ShareRowExclusiveLock. Two-phase: ADD CONSTRAINT ... NOT VALID; ALTER TABLE ... VALIDATE CONSTRAINT;",
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
            Statement::CreateIndex(idx) => {
                if !idx.concurrently {
                    v.push(MigViolation {
                        severity: MigSeverity::P0Block,
                        pattern: "create-index-not-concurrent",
                        fix: "CREATE INDEX without CONCURRENTLY blocks writes for the entire build duration. Use CREATE INDEX CONCURRENTLY — BUT it cannot run inside a transaction, and sqlx/most runners wrap each migration in one: mark the migration non-transactional (sqlx: `-- no-transaction` header) or split it out. A CONCURRENTLY build that fails leaves an INVALID index — DROP INDEX <name>; before retrying. (Idempotent re-runs: CREATE INDEX CONCURRENTLY IF NOT EXISTS.)",
                    });
                }
            }
            _ => {}
        }
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
        let src = "ALTER TABLE users DROP COLUMN email;";
        let r = detect("migrations_local/0002_drop.sql", src);
        assert!(r.iter().any(|v| v.pattern == "drop-column-direct"));
    }

    #[test]
    fn create_index_not_concurrent_blocked() {
        let src = "CREATE INDEX idx_users_email ON users (email);";
        let r = detect("migrate/0003_idx.sql", src);
        assert!(
            r.iter()
                .any(|v| v.pattern == "create-index-not-concurrent")
        );
    }

    #[test]
    fn create_index_concurrently_ok() {
        let src = "CREATE INDEX CONCURRENTLY idx_users_email ON users (email);";
        let r = detect("migrate/0003_idx.sql", src);
        assert!(
            !r.iter()
                .any(|v| v.pattern == "create-index-not-concurrent")
        );
    }

    #[test]
    fn fk_without_not_valid_blocked() {
        let src = "ALTER TABLE orders ADD CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users (id);";
        let r = detect("sqlx-migrations/0004_fk.sql", src);
        assert!(r.iter().any(|v| v.pattern == "fk-without-not-valid"));
    }

    #[test]
    fn fk_not_valid_ok() {
        let src = "ALTER TABLE orders ADD CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users (id) NOT VALID;";
        let r = detect("sqlx-migrations/0004_fk.sql", src);
        assert!(!r.iter().any(|v| v.pattern == "fk-without-not-valid"));
    }

    #[test]
    fn rename_column_blocked() {
        let src = "ALTER TABLE users RENAME COLUMN name TO full_name;";
        let r = detect("migrations/0005_rename.sql", src);
        assert!(r.iter().any(|v| v.pattern == "rename-column-direct"));
    }

    #[test]
    fn non_migration_file_skipped() {
        let src = "ALTER TABLE users DROP COLUMN email;";
        let r = detect("src/models.sql", src);
        assert!(r.is_empty());
    }

    #[test]
    fn create_index_fix_names_transaction_and_invalid_recovery() {
        let src = "CREATE INDEX idx_users_email ON users (email);";
        let r = detect("migrate/0003_idx.sql", src);
        let hit = r
            .iter()
            .find(|v| v.pattern == "create-index-not-concurrent")
            .unwrap();
        assert!(hit.fix.contains("CREATE INDEX CONCURRENTLY"));
        assert!(hit.fix.contains("sqlx: `-- no-transaction` header"));
        assert!(hit.fix.contains("DROP INDEX <name>;"));
    }

    #[test]
    fn commented_sql_is_ignored() {
        let src = "-- ALTER TABLE users DROP COLUMN email;\n/* CREATE INDEX idx_users_email ON users (email); */";
        let r = detect("migrations/0006_commented.sql", src);
        assert!(r.is_empty(), "Commented SQL must not trigger migration violations!");
    }
}

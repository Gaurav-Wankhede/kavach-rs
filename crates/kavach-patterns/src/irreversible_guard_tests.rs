use super::*;

#[test]
fn detects_drop_table() {
    let stmt = ["DROP", " TABLE users;"].concat();
    let v = detect("migrations/2026_05_24.sql", &stmt);
    assert!(v.iter().any(|h| h.pattern.contains("DROP")), "got: {v:?}");
}

#[test]
fn detects_truncate() {
    let stmt = ["TRUN", "CATE TABLE events;"].concat();
    let v = detect("migrations/wipe.sql", &stmt);
    assert!(v.iter().any(|h| h.pattern.contains("TRUNCATE")));
}

#[test]
fn detects_delete_without_where() {
    let stmt = ["DELE", "TE FROM users;"].concat();
    let v = detect("migrations/m.sql", &stmt);
    assert!(v.iter().any(|h| h.pattern.contains("DELETE without WHERE")));
}

#[test]
fn allows_delete_with_where() {
    let stmt = ["DELE", "TE FROM users WHERE id = 1;"].concat();
    let v = detect("migrations/m.sql", &stmt);
    assert!(
        !v.iter().any(|h| h.pattern.contains("DELETE without WHERE")),
        "false positive on WHERE-bound DELETE: {v:?}"
    );
}

#[test]
fn detects_alter_drop_column() {
    let stmt = ["ALT", "ER TABLE users DROP COLUMN email;"].concat();
    let v = detect("migrations/m.sql", &stmt);
    assert!(v.iter().any(|h| h.pattern.contains("DROP COLUMN")));
}

#[test]
fn flags_etc_path() {
    let v = detect("/etc/hosts", "127.0.0.1 localhost");
    assert!(v.iter().any(|h| h.pattern.contains("/etc")));
}

#[test]
fn flags_ssh_path() {
    let v = detect("/Users/x/.ssh/id_ed25519", "secret key bytes");
    assert!(v.iter().any(|h| h.pattern.contains(".ssh")));
}

#[test]
fn flags_migration_down() {
    let stmt = ["DROP", " TABLE old;"].concat();
    let v = detect("db/migrations/down/0001_users.sql", &stmt);
    assert!(v.iter().any(|h| h.pattern.contains("Migration down")));
}

#[test]
fn skips_test_files() {
    let stmt = ["DROP", " TABLE users;"].concat();
    let v = detect("crates/foo/src/tests/mod.rs", &stmt);
    assert!(v.is_empty(), "test files must be exempt: {v:?}");
}

#[test]
fn skips_non_sql_with_no_query_calls() {
    let v = detect("src/lib.rs", "fn drop_table() { }");
    assert!(
        v.is_empty(),
        "Rust source without sqlx::query must skip: {v:?}"
    );
}

#[test]
fn scans_rust_with_sqlx_query() {
    let stmt = ["sqlx::query(\"DROP", " TABLE users\")"].concat();
    let v = detect("src/db.rs", &stmt);
    assert!(v.iter().any(|h| h.pattern.contains("DROP")));
}

use super::destructive_sql_keyword;

#[test]
fn allows_read_and_write_ops() {
    assert!(destructive_sql_keyword("psql $DSN -c 'SELECT * FROM users'").is_none());
    assert!(destructive_sql_keyword("psql $DSN -c 'INSERT INTO t VALUES (1)'").is_none());
    assert!(destructive_sql_keyword("psql $DSN -c 'UPDATE t SET x=1 WHERE id=2'").is_none());
    assert!(destructive_sql_keyword("psql $DSN -c 'CREATE TABLE t (id int)'").is_none());
}

#[test]
fn blocks_destructive_ops() {
    assert_eq!(
        destructive_sql_keyword("psql $DSN -c 'DELETE FROM users WHERE id=1'"),
        Some("delete")
    );
    assert_eq!(
        destructive_sql_keyword("psql $DSN -c 'DROP TABLE users'"),
        Some("drop")
    );
    assert_eq!(
        destructive_sql_keyword("psql $DSN -c 'TRUNCATE audit_log'"),
        Some("truncate")
    );
    assert_eq!(
        destructive_sql_keyword("psql $DSN -c 'DROP DATABASE prod'"),
        Some("drop")
    );
}

#[test]
fn identifier_substrings_do_not_trigger() {
    assert!(destructive_sql_keyword("psql $DSN -c 'SELECT deleted_at FROM users'").is_none());
    assert!(destructive_sql_keyword("psql $DSN -c 'SELECT * FROM dropdown_options'").is_none());
    assert!(
        destructive_sql_keyword("psql $DSN -c 'SELECT truncate_log FROM settings'").is_none()
    );
}

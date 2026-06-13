//! `check_psql_blocked`: quote-aware command-position detection, operation-aware
//! verdict. A psql token inside a quoted arg is inert data. Real psql is ALLOWED
//! for read/insert/update/create and HARD-BLOCKED only for delete/drop/truncate.

use super::super::super::psql::check_psql_blocked;

#[test]
fn psql_token_inside_quoted_arg_is_inert() {
    // CWE-184 radius CLOSED: psql tokens inside quoted args are data, not a call.
    assert!(check_psql_blocked("rg -n 'foo|psql' src/").is_none());
    assert!(check_psql_blocked("grep '| psql' migrate.sh").is_none());
    assert!(check_psql_blocked("echo 'pipe to | psql here'").is_none());
    assert!(check_psql_blocked("echo \"run x | psql y later\"").is_none());
}

#[test]
fn psql_read_and_write_ops_are_allowed() {
    // Operation-aware: SELECT/INSERT/UPDATE/CREATE are not destructive.
    assert!(check_psql_blocked("psql -d mydb -c 'select 1'").is_none());
    assert!(check_psql_blocked("echo 'select 1' | psql mydb").is_none());
    assert!(check_psql_blocked("psql $DSN -c 'INSERT INTO t VALUES (1)'").is_none());
    assert!(check_psql_blocked("psql $DSN -c 'UPDATE t SET x=1 WHERE id=2'").is_none());
    assert!(check_psql_blocked("psql $DSN -c 'CREATE TABLE t (id int)'").is_none());
    // Unknown SQL piped from a file: no destructive keyword visible -> allowed.
    assert!(check_psql_blocked("cat q.sql |psql mydb").is_none());
}

#[test]
fn psql_destructive_ops_are_hard_blocked() {
    assert!(check_psql_blocked("psql $DSN -c 'DELETE FROM users WHERE id=1'").is_some());
    assert!(check_psql_blocked("psql $DSN -c 'DROP TABLE users'").is_some());
    assert!(check_psql_blocked("psql $DSN -c 'TRUNCATE audit_log'").is_some());
    assert!(check_psql_blocked("echo 'DROP DATABASE prod' | psql mydb").is_some());
}

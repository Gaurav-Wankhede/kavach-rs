//! `check_psql_blocked`: quote-aware command-position detection. A psql token
//! inside any quoted arg is inert data; real invocations HARD-BLOCK.

use super::super::super::psql::check_psql_blocked;

#[test]
fn psql_blocked_is_quote_aware_full_radius_closure() {
    // CWE-184 radius CLOSED: psql tokens inside quoted args are data.
    assert!(check_psql_blocked("rg -n 'foo|psql' src/").is_none());
    assert!(check_psql_blocked("grep '| psql' migrate.sh").is_none());
    assert!(check_psql_blocked("echo 'pipe to | psql here'").is_none());
    assert!(check_psql_blocked("echo \"run x | psql y later\"").is_none());
    // Real invocations still HARD-BLOCK (psql in command position).
    assert!(check_psql_blocked("psql -d mydb -c 'select 1'").is_some());
    assert!(check_psql_blocked("echo 'select 1' | psql mydb").is_some());
    assert!(check_psql_blocked("cat q.sql |psql mydb").is_some());
}

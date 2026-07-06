use super::*;

#[test]
fn blocks_update_no_where() {
    let sql = ["UPD", "ATE users SET name = 'x'"].concat();
    assert!(check("m.sql", &sql).is_some());
}

#[test]
fn allows_update_with_where() {
    let sql = ["UPD", "ATE users SET name = 'x' WHERE id = 1"].concat();
    assert!(check("m.sql", &sql).is_none());
}

#[test]
fn blocks_sql_concat() {
    let code = ["s.push_str(\"SE", "LECT * FR", "OM t\");"].concat();
    assert!(check("src/repo.rs", &code).is_some());
}

#[test]
fn skips_tests() {
    let sql = ["UPD", "ATE t SET x = 1"].concat();
    assert!(check("src/tests/t.rs", &sql).is_none());
}

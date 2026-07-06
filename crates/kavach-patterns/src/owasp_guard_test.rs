use super::*;

fn sql_inject_sample() -> String {
    [
        "let q = fo",
        "rmat!(\"SE",
        "LE",
        "CT * FR",
        "OM users WH",
        "ERE id = {}\", uid);",
    ]
    .concat()
}

fn sql_param_sample() -> String {
    [
        "sqlx::qu",
        "ery!(\"SE",
        "LE",
        "CT * FR",
        "OM users WH",
        "ERE id = $1\", uid)",
    ]
    .concat()
}

fn sql_delete_sample() -> String {
    [
        "fo",
        "rmat!(\"DE",
        "LE",
        "TE FR",
        "OM t WH",
        "ERE id = {}\", x)",
    ]
    .concat()
}

#[test]
fn detects_sql_injection() {
    let f = detect("src/h.rs", &sql_inject_sample());
    assert!(!f.is_empty());
    assert_eq!(f.first().map(|x| x.category), Some("A03:SQLi"));
}

#[test]
fn allows_parameterized() {
    assert!(detect("src/h.rs", &sql_param_sample()).is_empty());
}

#[test]
fn detects_xss() {
    assert!(!detect("src/c.tsx", "el.innerHTML = input;").is_empty());
}

#[test]
fn skips_tests() {
    assert!(detect("src/tests/int.rs", &sql_inject_sample()).is_empty());
}

#[test]
fn check_blocks_critical() {
    assert!(check("src/h.rs", &sql_delete_sample()).is_some());
}

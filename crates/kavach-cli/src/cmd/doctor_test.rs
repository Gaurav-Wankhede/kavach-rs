//! Tests for the doctor self-audit matrix.
use super::detect::{Class, scan_source};

#[test]
fn flags_let_underscore_on_io() {
    let f = scan_source("x.rs", "        let _ = cmd.output();");
    assert!(f.iter().any(|x| x.class == Class::SilentDiscard));
}

#[test]
fn ignores_let_underscore_on_pure_value() {
    // A discard of a non-IO value (e.g. writeln to a String) is not flagged.
    let f = scan_source("x.rs", "        let _ = writeln!(buf, \"hi\");");
    assert!(!f.iter().any(|x| x.class == Class::SilentDiscard));
}

#[test]
fn flags_destructive_query_string() {
    // Unbounded DELETE (no key predicate) is the flagged class.
    let f = scan_source("x.rs", r#"    db.query("DELETE pattern")"#);
    assert!(f.iter().any(|x| x.class == Class::DestructiveQuery));
}

#[test]
fn does_not_flag_update_only_a_delete() {
    // UPDATE mutates fields, not the unbounded-delete class — must not flag.
    let f = scan_source("x.rs", r#"    db.query("UPDATE decision SET priority = $p WHERE id = $i")"#);
    assert!(!f.iter().any(|x| x.class == Class::DestructiveQuery));
}

#[test]
fn does_not_flag_bounded_delete() {
    // A DELETE bounded by a key predicate is already targeted.
    let f = scan_source("x.rs", r#"    db.query("DELETE pattern WHERE project = $pid AND entry_key = $key")"#);
    assert!(!f.iter().any(|x| x.class == Class::DestructiveQuery));
}

#[test]
fn flags_unbounded_delete() {
    let f = scan_source("x.rs", r#"    db.query("DELETE event")"#);
    assert!(f.iter().any(|x| x.class == Class::DestructiveQuery));
}

#[test]
fn does_not_flag_delete_in_test_file() {
    // A DELETE fixture in a *_test.rs is not a production mutation.
    let f = scan_source("foo_test.rs", r#"    db.query("DELETE event")"#);
    assert!(!f.iter().any(|x| x.class == Class::DestructiveQuery));
}

#[test]
fn does_not_flag_sql_guard_pattern_literals() {
    // kavach's own sql_destructive guard stores "DELETE " as detection DATA.
    let f = scan_source(
        "crates/kavach-engine/src/gates/sql_destructive.rs",
        r#"    const BANNED: &str = "DELETE ";"#,
    );
    assert!(!f.iter().any(|x| x.class == Class::DestructiveQuery));
}

#[test]
fn flags_swallowed_err_arm() {
    let f = scan_source("x.rs", "            Ok(None) | Err(_) => None,");
    assert!(f.iter().any(|x| x.class == Class::SwallowedArm));
}

#[test]
fn does_not_flag_err_arm_that_logs() {
    let f = scan_source("x.rs", "            Err(_) => { tracing::warn!(\"x\"); None }");
    assert!(!f.iter().any(|x| x.class == Class::SwallowedArm));
}

#[test]
fn doctor_ok_marker_silences_a_line() {
    // A reviewed benign-by-design site is silenced explicitly, not by dropping
    // the class — the whole point of the escape.
    let f = scan_source(
        "x.rs",
        r#"        db.query("DELETE x WHERE 1=1") // doctor:ok benign fixture"#,
    );
    assert!(f.is_empty(), "doctor:ok must silence the line");
}

#[test]
fn comment_prose_does_not_trip_code_matchers() {
    // A `let _ =` mentioned in a comment is prose, not code.
    let f = scan_source("x.rs", "        // historically: let _ = cmd.output() was a bug");
    assert!(f.is_empty(), "comment mention must not be flagged");
}

#[test]
fn one_based_line_numbers() {
    let src = "fn a() {}\nfn b() {}\n            Err(_) => None,";
    let f = scan_source("x.rs", src);
    assert_eq!(f[0].line, 3, "1-based line of the finding");
}

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
    let f = scan_source("x.rs", r#"    db.query("DELETE pattern WHERE project = $pid")"#);
    assert!(f.iter().any(|x| x.class == Class::DestructiveQuery));
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

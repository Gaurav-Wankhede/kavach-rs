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
fn does_not_flag_delete_bound_by_contains_param() {
    // A DELETE scoped by `CONTAINS $param` is bounded by that param — not the
    // unbounded class. The frozen `= $`/`$key`-only check was the FP source.
    let f = scan_source(
        "x.rs",
        r#"    let q = "DELETE entity WHERE entity_type = 'x' AND props.gate CONTAINS $gate RETURN BEFORE";"#,
    );
    assert!(!f.iter().any(|x| x.class == Class::DestructiveQuery));
}

#[test]
fn does_not_flag_delete_with_return_before_readback() {
    // `RETURN BEFORE` is the count→delete→verify read-back the check asks for;
    // a DELETE that returns its deleted rows is verified, not silent.
    let f = scan_source("x.rs", r#"    db.query("DELETE event RETURN BEFORE")"#);
    assert!(!f.iter().any(|x| x.class == Class::DestructiveQuery));
}

#[test]
fn still_flags_unbounded_delete_without_param_or_readback() {
    // Guard the fix doesn't over-widen: a bare DELETE with a literal-only WHERE
    // and no bound param and no RETURN BEFORE is still unbounded.
    let f = scan_source("x.rs", r#"    db.query("DELETE event WHERE status = 'stale'")"#);
    assert!(f.iter().any(|x| x.class == Class::DestructiveQuery));
}

#[test]
fn does_not_flag_multiline_delete_with_continuation_param() {
    // A `\`-continued string literal carries the WHERE `$param` + `RETURN BEFORE`
    // onto a later physical line; the scanner must join the logical statement.
    let src = "    let q = \"DELETE entity WHERE entity_type = 'mistake_event' \\\n                 AND props.gate CONTAINS $gate RETURN BEFORE\";";
    let f = scan_source("x.rs", src);
    assert!(!f.iter().any(|x| x.class == Class::DestructiveQuery));
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

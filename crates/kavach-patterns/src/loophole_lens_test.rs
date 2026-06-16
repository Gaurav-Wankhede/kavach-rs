//! Tests for the shared loophole lens kernel.

use super::{Lens, classify, scan_text};

#[test]
fn all_six_lenses_have_unique_slugs() {
    let mut slugs: Vec<&str> = Lens::ALL.iter().map(|l| l.slug()).collect();
    assert_eq!(slugs.len(), 6);
    slugs.sort_unstable();
    slugs.dedup();
    assert_eq!(slugs.len(), 6, "lens slugs must be unique");
}

#[test]
fn flags_discarded_result_as_failure() {
    let f = scan_text("let _ = fallible()?;");
    assert!(f.iter().any(|x| x.lens == Lens::Failure));
}

#[test]
fn flags_unwrap_as_malformed() {
    let f = scan_text("let v = parse(input).unwrap();");
    assert!(f.iter().any(|x| x.lens == Lens::Malformed));
}

#[test]
fn flags_index_zero_as_boundary() {
    let f = scan_text("let head = items[0];");
    assert!(f.iter().any(|x| x.lens == Lens::Boundary));
}

#[test]
fn ignores_comment_lines() {
    let f = scan_text("// never write let _ = foo()? here");
    assert!(f.is_empty(), "comments are guidance, not code");
}

#[test]
fn clean_line_yields_nothing() {
    let f = scan_text("let sum = a.checked_add(b)?;");
    assert!(f.is_empty());
}

#[test]
fn one_line_yields_at_most_one_finding() {
    let f = scan_text("let v = items[0].unwrap();");
    assert_eq!(f.len(), 1, "one line, one (lens,site) finding");
}

#[test]
fn stops_at_cfg_test_boundary() {
    let src = "fn prod() {}\n#[cfg(test)]\nmod t {\n  let v = x.unwrap();\n}\n";
    assert!(scan_text(src).is_empty(), "test code excluded");
}

#[test]
fn classify_first_match_wins() {
    // unwrap + index on one line → failure-order: malformed precedes boundary.
    let (lens, _) = classify("let v = items[0].unwrap();").unwrap();
    assert_eq!(lens, Lens::Malformed, "first matching lens wins");
}

#[test]
fn line_numbers_are_one_based() {
    let src = "fn ok() {}\nlet v = x.unwrap();\n";
    let f = scan_text(src);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].line, 2, "1-based line of the unwrap");
}

// ---- replay lens: SQL INSERT yes, local .insert() no (FP refinement) ----

#[test]
fn replay_fires_on_sql_insert() {
    let (lens, _) = classify(r#"let q = "INSERT INTO t (a) VALUES (?)";"#).unwrap();
    assert_eq!(lens, Lens::Replay);
}

#[test]
fn replay_silent_on_local_collection_insert() {
    // A Rust HashMap/HashSet/Vec .insert() is in-memory mutation, NOT a
    // non-idempotent persisted write — must not fire (the gates/loader FP class).
    assert!(classify("seen.insert(key);").is_none(), "local .insert() is not replay");
    assert!(classify("map.insert(k, v);").is_none());
}

#[test]
fn replay_silent_on_idempotent_sql() {
    assert!(classify("UPSERT INTO t ...").is_none(), "upsert is idempotent");
    assert!(classify("INSERT INTO t ... ON CONFLICT DO NOTHING").is_none());
}

// ---- concurrency lens: DB existence check yes, local .contains() no ----

#[test]
fn concurrency_fires_on_existence_check() {
    let (lens, _) = classify("if !row_exists(id) { write(id) }").unwrap();
    assert_eq!(lens, Lens::Concurrency);
}

#[test]
fn concurrency_silent_on_local_contains() {
    // Single-threaded membership on a local collection is not a cross-actor
    // TOCTOU (the loophole.rs/dag_render FP class).
    assert!(classify("if !keys.contains(&incident) { keys.push(incident) }").is_none());
}

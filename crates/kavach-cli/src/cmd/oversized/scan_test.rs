//! `has_exempt_marker` honors the same intentional-ceiling markers as the live
//! nano-file PreWrite guard, so a file the guard lets pass is not re-flagged here.
use super::has_exempt_marker;
use std::fs;

fn write_tmp(name: &str, body: &str) -> String {
    let dir = std::env::temp_dir().join(format!("kavach-oversized-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("mk tmp dir");
    let path = dir.join("f.rs");
    fs::write(&path, body).expect("write tmp file");
    path.to_string_lossy().into_owned()
}

#[test]
fn split_marker_is_exempt() {
    let p = write_tmp("split", "// split: intentional hub\nfn x() {}\n");
    assert!(has_exempt_marker(&p));
}

#[test]
fn intentional_marker_is_exempt() {
    // The marker the nano-file guard added (kavach:intentional) must also exempt
    // a file from the oversized scan — the two gates stay consistent.
    let p = write_tmp("intentional", "// kavach:intentional one exhaustive match\nfn x() {}\n");
    assert!(has_exempt_marker(&p));
}

#[test]
fn no_marker_is_not_exempt() {
    let p = write_tmp("plain", "fn x() {}\nfn y() {}\n");
    assert!(!has_exempt_marker(&p));
}

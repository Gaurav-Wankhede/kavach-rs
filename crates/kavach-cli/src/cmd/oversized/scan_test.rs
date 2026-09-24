//! `has_exempt_marker` honors intentional-ceiling markers so flagged files are
//! not re-flagged by the oversized scan.
use super::has_exempt_marker;
use std::fs;

fn write_tmp(name: &str, body: &str) -> String {
    let dir = std::env::temp_dir().join(format!("kavach-oversized-{name}"));
    drop(fs::remove_dir_all(&dir));
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
    // The marker (kavach:intentional) must also exempt
    // a file from the oversized scan — consistent behavior.
    let p = write_tmp(
        "intentional",
        "// kavach:intentional one exhaustive match\nfn x() {}\n",
    );
    assert!(has_exempt_marker(&p));
}

#[test]
fn no_marker_is_not_exempt() {
    let p = write_tmp("plain", "fn x() {}\nfn y() {}\n");
    assert!(!has_exempt_marker(&p));
}

#[test]
fn marker_inside_a_string_is_not_an_escape_hatch() {
    let p = write_tmp(
        "strdecoy",
        "const M: &str = \"// split: not really\";\nfn x() {}\n",
    );
    assert!(!has_exempt_marker(&p));
}

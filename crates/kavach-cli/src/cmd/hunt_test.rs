use super::registry::scan_file;
use super::run;

#[test]
fn finds_silent_io_worst_practice() {
    let hits = scan_file("crates/x/src/h.rs", "pub fn f() {\n    let _ = do_io();\n}\n");
    assert!(
        hits.iter().any(|f| f.detector == "silent_io"),
        "silent-IO let _ = must be flagged"
    );
}

#[test]
fn clean_file_has_no_findings() {
    let hits = scan_file("crates/x/src/h.rs", "pub fn f() -> u8 {\n    1\n}\n");
    assert!(hits.is_empty(), "a clean file yields no worst-practice hits");
}

#[test]
fn run_on_missing_path_exits_2() {
    let missing = std::path::Path::new("/nonexistent/kavach/hunt/path");
    assert_eq!(run(missing), 2, "unreadable root must exit 2, never silent 0");
}

#[test]
fn run_on_clean_tmp_dir_exits_0() {
    let dir = std::env::temp_dir().join(format!("kavach_hunt_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mk tmp dir");
    std::fs::write(dir.join("clean.rs"), "pub fn f() -> u8 { 1 }\n").expect("seed clean");
    assert_eq!(run(&dir), 0, "a clean tree exits 0");
    std::fs::remove_dir_all(&dir).ok();
}

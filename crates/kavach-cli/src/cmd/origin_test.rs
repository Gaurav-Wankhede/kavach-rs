use super::run;

#[test]
fn run_on_missing_path_exits_2() {
    assert_eq!(run("FOO", std::path::Path::new("/nonexistent/x"), false), 2);
}

#[test]
fn run_with_empty_name_exits_2() {
    assert_eq!(run("", std::path::Path::new("."), false), 2);
}

#[test]
fn finds_const_origin_in_tmp_tree() {
    let dir = std::env::temp_dir().join(format!("kavach_origin_{}", std::process::id()));
    std::fs::create_dir_all(dir.join("src")).expect("mk tmp");
    std::fs::write(
        dir.join("src/config.rs"),
        "pub const MAX_RETRIES: u32 = 5;\n",
    )
    .expect("seed");
    assert_eq!(run("MAX_RETRIES", &dir, false), 0, "a declared const is found (exit 0)");
    assert_eq!(run("NOPE_MISSING", &dir, false), 1, "an absent symbol exits 1");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn resolves_in_single_file() {
    let file = std::env::temp_dir().join(format!("kavach_single_{}.rs", std::process::id()));
    std::fs::write(&file, "pub const FOO: u8 = 1;\npub fn connect(timeout: u32) {}\npub const MAXN: u8 = 3;\n").expect("write");
    assert_eq!(run("FOO", &file, false), 0, "const FOO found in single file");
    assert_eq!(run("connect", &file, false), 0, "fn connect found in single file");
    assert_eq!(run("MAXN", &file, false), 0, "const MAXN found in single file");
    assert_eq!(run("MISSING", &file, false), 1, "missing symbol exits 1");
    std::fs::remove_file(&file).ok();
}

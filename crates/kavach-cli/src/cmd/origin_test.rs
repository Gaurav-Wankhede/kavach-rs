use super::run;

#[test]
fn run_on_missing_path_exits_2() {
    assert_eq!(run("FOO", std::path::Path::new("/nonexistent/x")), 2);
}

#[test]
fn run_with_empty_name_exits_2() {
    assert_eq!(run("", std::path::Path::new(".")), 2);
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
    assert_eq!(run("MAX_RETRIES", &dir), 0, "a declared const is found (exit 0)");
    assert_eq!(run("NOPE_MISSING", &dir), 1, "an absent symbol exits 1");
    std::fs::remove_dir_all(&dir).ok();
}

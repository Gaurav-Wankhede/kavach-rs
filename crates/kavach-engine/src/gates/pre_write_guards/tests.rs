//! Nano-file split detector regression tests for the `PreWrite` guard dispatch.
use crate::gates::pre_write_context::WriteContext;
use kavach_types::HookInput;
use std::collections::HashMap;

// Regression: the nano-file split detector must see the WHOLE post-edit
// file, not just the Edit fragment. Before the fix, the dispatch passed
// ctx.content (= new_string), so an Edit that left a 200-LOC file in place
// never tripped the >100-LOC split advisory — it only fired for Write.
// This proves the guard runs over ctx.effective_content (full file body)
// and flags an oversized file edited in place.
#[test]
fn edit_on_oversized_file_trips_nano_file_split_detector() {
    let dir = std::env::temp_dir().join(format!("kavach_micro_edit_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mk tmp dir");
    let path = dir.join("oversized.rs");
    // 200 lines on disk; the Edit fragment below is two lines.
    let body = "fn line() {}\n".repeat(200);
    std::fs::write(&path, &body).expect("seed file");
    let path_str = path.to_string_lossy().into_owned();

    let mut input = HookInput::default();
    input.tool_name = "Edit".into();
    input.tool_input = Some(HashMap::from([
        ("file_path".into(), serde_json::json!(path_str)),
        ("old_string".into(), serde_json::json!("fn line() {}")),
        ("new_string".into(), serde_json::json!("fn patched() {}")),
    ]));

    let ctx = WriteContext::extract(&input);
    // effective_content holds the full 200-line file; the fragment is short.
    assert!(ctx.effective_content.lines().count() >= 200);
    assert!(ctx.content.lines().count() < 5);

    // The exact call the dispatch makes — must flag the oversized file.
    let hits = kavach_patterns::nano_file_guard::detect(
        ctx.file_path,
        &ctx.effective_content,
        ctx.tool_name,
    );
    assert!(
        hits.iter().any(|v| v.pattern.contains("100 LOC")),
        "Edit on a 200-LOC file must trip the nano-file split detector"
    );
    // An in-place Edit that keeps the file over 100 LOC HARD-BLOCKS too: it must
    // split into the same deep hub+leaf hierarchy as a new file (Rust 2024:
    // foo.rs + foo/ with pub use re-exports, no mod.rs).
    assert!(
        hits.iter().any(|v| v.pattern == "file exceeds 100 LOC"
            && v.severity == kavach_patterns::nano_file_guard::NanoSeverity::P0Block),
        "an in-place Edit over 100 LOC must P0-block (forces the split)"
    );

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&dir).ok();
}

// Counterpart: feeding only the short fragment (the OLD buggy behaviour)
// would NOT flag — locking in that effective_content is what fixes it.
#[test]
fn fragment_only_would_miss_oversized_file() {
    let fragment = "fn patched() {}\n";
    let hits =
        kavach_patterns::nano_file_guard::detect("crates/foo/src/oversized.rs", fragment, "Edit");
    assert!(
        !hits.iter().any(|v| v.pattern.contains("100 LOC")),
        "a 2-line fragment is under 100 LOC — proves why the fragment path missed it"
    );
}

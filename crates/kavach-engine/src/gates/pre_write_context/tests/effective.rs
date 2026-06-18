//! `effective_content` population: Edit reads the full file body, falls back to
//! the fragment when the file is missing, and Write mirrors `content`.
use crate::gates::pre_write_context::WriteContext;
use kavach_types::HookInput;

// ALGO: `SingleSyncReadOnce`; PROBLEM_CLASS io. Rejected `memmap2` (per-call
// overhead > fs::read for <1MB). TIME O(file_bytes); one fs read per Edit hook.
#[test]
fn edit_populates_effective_content_with_full_file_body() {
    let dir = std::env::temp_dir().join(format!("kavach_eff_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("big.rs");
    let body: String = (0..250).fold(String::new(), |mut acc, i| {
        use std::fmt::Write;
        writeln!(acc, "// row {i}").ok();
        acc
    });
    std::fs::write(&path, &body).unwrap();
    let path_str = path.to_string_lossy().into_owned();

    let mut input = HookInput::default();
    input.tool_name = "Edit".into();
    input.tool_input = Some(std::collections::HashMap::from([
        ("file_path".into(), serde_json::json!(path_str)),
        ("old_string".into(), serde_json::json!("// row 0")),
        ("new_string".into(), serde_json::json!("// patched")),
    ]));
    let ctx = WriteContext::extract(&input);
    assert_eq!(ctx.content, "// patched");
    assert_eq!(ctx.effective_content.lines().count(), 250);
    // POST-EDIT body: the old_string->new_string substitution is applied, so a
    // downstream guard judges the RESULT, not the stale pre-edit file.
    assert!(
        ctx.effective_content.contains("// patched"),
        "effective_content must reflect the post-edit substitution"
    );

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&dir).ok();
}

// The catch-22 fix: an Edit that ADDS the //kavach:micro-file-exempt marker as a
// new first line must surface that marker in the post-edit body — else you could
// never add it to an already-oversize file (the edit adding it would be blocked
// because the marker isn't present yet). SOURCE: mistake.edit-effective-content-*.
#[test]
fn edit_adding_exempt_marker_appears_in_effective_body() {
    let dir = std::env::temp_dir().join(format!("kavach_marker_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("root.rs");
    std::fs::write(&path, "// header\npub mod a;\n").unwrap();
    let path_str = path.to_string_lossy().into_owned();

    let mut input = HookInput::default();
    input.tool_name = "Edit".into();
    input.tool_input = Some(std::collections::HashMap::from([
        ("file_path".into(), serde_json::json!(path_str)),
        ("old_string".into(), serde_json::json!("// header")),
        (
            "new_string".into(),
            serde_json::json!("// kavach:micro-file-exempt — crate root\n// header"),
        ),
    ]));
    let ctx = WriteContext::extract(&input);
    assert!(
        ctx.effective_content.contains("kavach:micro-file-exempt"),
        "marker added by the edit must be in the post-edit body"
    );

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&dir).ok();
}

#[test]
fn edit_falls_back_to_content_when_file_missing() {
    let mut input = HookInput::default();
    input.tool_name = "Edit".into();
    input.tool_input = Some(std::collections::HashMap::from([
        (
            "file_path".into(),
            serde_json::json!("/nonexistent/path/abc123.rs"),
        ),
        ("new_string".into(), serde_json::json!("fragment")),
    ]));
    let ctx = WriteContext::extract(&input);
    assert_eq!(ctx.effective_content, "fragment");
}

// FAIL-CLOSED: file exists but `old_string` does NOT match it → the true
// post-edit body is unknown. The guard must NOT undercount by judging the tiny
// fragment; it returns the larger {current file} so a downstream LOC cap still
// sees the oversized file. SOURCE: <https://devsecopsschool.com/blog/fail-safe-defaults/>
#[test]
fn edit_unmatched_old_string_fails_closed_to_larger_body() {
    let dir = std::env::temp_dir().join(format!("kavach_failclosed_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("oversize.rs");
    let body: String = (0..200).fold(String::new(), |mut acc, i| {
        use std::fmt::Write;
        writeln!(acc, "// line {i}").ok();
        acc
    });
    std::fs::write(&path, &body).unwrap();
    let path_str = path.to_string_lossy().into_owned();

    let mut input = HookInput::default();
    input.tool_name = "Edit".into();
    input.tool_input = Some(std::collections::HashMap::from([
        ("file_path".into(), serde_json::json!(path_str)),
        // old_string that is NOT present in the file → reconstruction impossible.
        (
            "old_string".into(),
            serde_json::json!("// THIS DOES NOT EXIST"),
        ),
        ("new_string".into(), serde_json::json!("// tiny")),
    ]));
    let ctx = WriteContext::extract(&input);
    // Worst-case body returned (the 200-line file), NOT the 1-line fragment, so a
    // 100-LOC guard still blocks. Pre-fix this returned the file too, but an
    // empty old_string returned `current` unconditionally; this proves the
    // larger-of-the-two rule for the unmatched case explicitly.
    assert_eq!(
        ctx.effective_content.lines().count(),
        200,
        "unmatched edit must fail closed to the larger (full-file) body"
    );

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&dir).ok();
}

// FAIL-CLOSED edge: empty file on disk + a multi-line fragment whose old_string
// is absent → the FRAGMENT is larger, so it (not the empty file) is judged.
#[test]
fn edit_unmatched_picks_fragment_when_fragment_is_larger() {
    let dir = std::env::temp_dir().join(format!("kavach_fc2_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("tiny.rs");
    std::fs::write(&path, "// one\n").unwrap();
    let path_str = path.to_string_lossy().into_owned();

    let big_fragment = "// a\n// b\n// c\n// d\n// e";
    let mut input = HookInput::default();
    input.tool_name = "Edit".into();
    input.tool_input = Some(std::collections::HashMap::from([
        ("file_path".into(), serde_json::json!(path_str)),
        ("old_string".into(), serde_json::json!("// NOPE")),
        ("new_string".into(), serde_json::json!(big_fragment)),
    ]));
    let ctx = WriteContext::extract(&input);
    assert_eq!(
        ctx.effective_content.lines().count(),
        5,
        "fragment larger than file → judge the fragment (worst case)"
    );

    std::fs::remove_file(&path).ok();
    std::fs::remove_dir(&dir).ok();
}

#[test]
fn write_sets_effective_content_to_content() {
    let mut input = HookInput::default();
    input.tool_name = "Write".into();
    input.tool_input = Some(std::collections::HashMap::from([
        ("file_path".into(), serde_json::json!("src/new.rs")),
        ("content".into(), serde_json::json!("fn main() {}")),
    ]));
    let ctx = WriteContext::extract(&input);
    assert_eq!(ctx.effective_content, "fn main() {}");
}

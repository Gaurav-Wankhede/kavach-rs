//! Depth-resolution + content-truncation proofs for `kavach db query --depth`.
use super::{Depth, render_content, resolve_depth};

#[test]
fn no_flag_no_env_is_none() {
    assert_eq!(resolve_depth(None, false), Depth::None);
}

#[test]
fn env_override_forces_all() {
    assert_eq!(resolve_depth(None, true), Depth::All);
    assert_eq!(resolve_depth(Some("40"), true), Depth::All, "env wins over a char flag");
}

#[test]
fn all_keyword_is_all() {
    assert_eq!(resolve_depth(Some("all"), false), Depth::All);
    assert_eq!(resolve_depth(Some("ALL"), false), Depth::All, "case-insensitive");
}

#[test]
fn integer_flag_is_chars() {
    assert_eq!(resolve_depth(Some("400"), false), Depth::Chars(400));
    assert_eq!(resolve_depth(Some("  12 "), false), Depth::Chars(12), "trimmed");
}

#[test]
fn non_integer_flag_fails_safe_to_none() {
    // A typo must NOT dump the whole body — fail-safe to titles-only.
    assert_eq!(resolve_depth(Some("lots"), false), Depth::None);
    assert_eq!(resolve_depth(Some("-5"), false), Depth::None, "negative is not usize");
}

#[test]
fn none_renders_no_content() {
    assert_eq!(render_content("anything", Depth::None), None);
}

#[test]
fn all_renders_whole_body() {
    assert_eq!(
        render_content("full body here", Depth::All),
        Some("full body here".to_owned())
    );
}

#[test]
fn chars_truncates_and_marks_when_cut() {
    let out = render_content("abcdefghij", Depth::Chars(4)).expect("some");
    assert!(out.starts_with("abcd"), "first 4 chars kept: {out}");
    assert!(out.contains("truncated"), "cut bodies get a marker: {out}");
}

#[test]
fn chars_no_marker_when_body_fits() {
    assert_eq!(
        render_content("abc", Depth::Chars(10)),
        Some("abc".to_owned()),
        "a body shorter than the cap is printed whole with no marker"
    );
}

#[test]
fn chars_respects_utf8_char_boundary() {
    // 'é' is 2 bytes; Chars(2) must keep 2 CHARS without panicking on a byte split.
    let out = render_content("aérn", Depth::Chars(2)).expect("some");
    assert!(out.starts_with("aé"), "kept 2 chars across a multi-byte char: {out}");
}

#[test]
fn chars_zero_keeps_nothing_but_marks_truncation() {
    let out = render_content("xyz", Depth::Chars(0)).expect("some");
    assert!(out.contains("truncated"), "depth 0 on a non-empty body marks a cut: {out}");
}

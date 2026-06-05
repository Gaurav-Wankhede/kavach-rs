//! `check_multi_crate` tests: multi-`-p` warn, single-`-p` allow, value-flag and
//! end-of-options exemptions, quoted-arg exemption, plus the strip primitive.
use super::super::check_multi_crate;
use crate::gates::pre_tool_bash::strip_quoted_regions;

#[test]
fn should_warn_when_multiple_p_flags_on_cargo_check() {
    assert!(check_multi_crate("cargo check -p soundbak-social-service -p user-service").is_some());
}

#[test]
fn should_not_warn_for_single_p_flag() {
    assert!(check_multi_crate("cargo check -p kavach-engine").is_none());
}

#[test]
fn single_p_with_filter_expr_value_is_not_multi_crate() {
    // One crate + an `-E` filter whose value contains a `-p` token must NOT trip.
    assert!(check_multi_crate("cargo check -p kavach-engine -E 'test(handles_-p_case)'").is_none());
    assert!(check_multi_crate("cargo build -p kavach-engine --message-format -p").is_none());
    assert!(
        check_multi_crate("cargo check --manifest-path ./-p/Cargo.toml -p kavach-engine").is_none()
    );
}

#[test]
fn p_token_after_end_of_options_marker_is_not_counted() {
    assert!(check_multi_crate("cargo check -p a -- -p").is_none());
}

#[test]
fn glued_package_form_is_counted() {
    assert!(check_multi_crate("cargo check -p=a --package=b").is_some());
}

#[test]
fn genuine_two_package_flags_still_warn_after_fix() {
    // FP-bound guard: the fix must not silence the real violation.
    assert!(check_multi_crate("cargo check -p a -p b").is_some());
    assert!(
        check_multi_crate("cargo build -p a -E 'test(x)' -p b --message-format json").is_some()
    );
}

#[test]
fn cargo_text_inside_quoted_commit_message_is_not_an_invocation() {
    // The advisory must not fire on a commit whose -m body documents cargo prose.
    let c = "git commit -m \"fix: run cargo check -p crate-a and cargo check -p crate-b\"";
    assert!(check_multi_crate(c).is_none());
    assert!(check_multi_crate("echo 'cargo build -p a -p b'").is_none());
    // Real invocation chained after a quoted commit still warns.
    assert!(check_multi_crate("git commit -m \"msg\" && cargo check -p a -p b").is_some());
}

#[test]
fn strip_quoted_regions_collapses_span_to_one_token() {
    // Quoted text gone; span is exactly one token so a preceding value-flag
    // consumes the placeholder, not the next real flag.
    let s = strip_quoted_regions("a 'b b' c");
    assert!(!s.contains('b'));
    assert_eq!(s.split_whitespace().count(), 3, "got {s:?}");
    let e = strip_quoted_regions("-E 'test(x)' -p kavach-engine");
    let toks: Vec<&str> = e.split_whitespace().collect();
    assert_eq!(toks, ["-E", "_", "-p", "kavach-engine"], "got {e:?}");
}

// Enforcement-teeth proofs: distinctive_marker bounds + carve-outs.
// The live RPC path (fetch_retired) is exercised end-to-end via the daemon, not
// here; these lock the pure-logic guards that decide WHETHER to match.
use super::distinctive_marker;

#[test]
fn marker_strips_name_half_and_lowercases() {
    // "name: rationale" → name half only, lowercased.
    let m = distinctive_marker("dioxus-0.7 web-sys gap: route via BFF").expect("long enough");
    assert!(m.starts_with("dioxus"), "{m}");
    assert!(!m.contains("route via"), "rationale dropped: {m}");
    assert_eq!(m, m.to_lowercase(), "lowercased");
}

#[test]
fn marker_rejects_short_generic_titles() {
    // Below MIN_DISTINCTIVE_LEN ⇒ None, so a generic short word never matches.
    assert!(distinctive_marker("env").is_none());
    assert!(distinctive_marker("use mock").is_none());
}

#[test]
fn marker_keeps_long_distinctive_head() {
    let m = distinctive_marker("XInternalSecret header auth").expect("long");
    assert_eq!(m, "xinternalsecret header auth");
}

//! Red-Green proofs for the RPC witness-receipt gate decision logic.

use super::{Receipt, decide};

fn good(head: &str, ts: i64, sess: &str) -> Receipt {
    Receipt::new(true, head.to_owned(), ts, sess.to_owned())
}

#[test]
fn non_roadmap_promotion_is_not_gated_even_without_receipt() {
    // A decision card or a non-completion status needs no receipt.
    assert!(decide("decision", "verified", None, "HEAD", 0, "s").is_none());
    assert!(decide("roadmap", "in_progress", None, "HEAD", 0, "s").is_none());
}

#[test]
fn roadmap_completion_without_receipt_is_refused() {
    let msg = decide("roadmap", "verified", None, "HEAD", 0, "s")
        .expect("missing receipt must refuse a gated promotion");
    assert!(msg.contains("REFUSED"), "names the refusal: {msg}");
    assert!(msg.contains("witness"), "points at the witness: {msg}");
}

#[test]
fn roadmap_completion_with_valid_receipt_is_allowed() {
    let r = good("abc123", 1_000, "sess-1");
    assert!(decide("roadmap", "verified", Some(&r), "abc123", 1_000, "sess-1").is_none());
}

#[test]
fn stale_or_mismatched_receipt_is_refused_with_reason() {
    let r = good("oldsha", 1_000, "sess-1");
    let msg = decide("roadmap", "done", Some(&r), "newsha", 1_000, "sess-1")
        .expect("HEAD mismatch must refuse");
    assert!(msg.contains("REFUSED"), "names refusal: {msg}");
    assert!(msg.contains("HEAD"), "surfaces the specific reason: {msg}");
}

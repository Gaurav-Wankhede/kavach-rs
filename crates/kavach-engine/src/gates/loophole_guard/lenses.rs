//! Brain-OS-sourced, risk-adaptive loophole lenses.
//!
//! The loophole interrogation no longer injects ONE frozen 6-lens list. Instead
//! it classifies the changed content's risk surface (auth / crypto / money /
//! concurrency / persistence / …) from the markers that fired, asks Brain-OS for
//! the loophole dimensions that matter for THAT surface, and injects those. A
//! crypto change is interrogated on key-management / side-channel / nonce-reuse;
//! a money path on rounding / precision / idempotency — multi-dimensional and
//! autonomous, not one-size-fits-all. Fail-soft: an empty corpus / daemon blip
//! yields the canonical six, so the gate is never weaker than before.
//! SOURCE: decision.loophole-lenses-brain-os-dynamic.

/// The canonical six lenses — the fail-soft floor served verbatim when Brain-OS
/// surfaces nothing for the risk surface. These are the universal dimensions; the
/// dynamic path ADDS surface-specific ones on top, never drops these.
pub(super) const CANONICAL_LENSES: &str =
    "- concurrency: two actors at once -> TOCTOU / lost-update / double-claim. \
     CLOSE with an atomic/compare-and-swap/lock, then cite it.\n\
     - failure: process dies mid-op -> orphaned lock / half-write / leaked task. \
     CLOSE with a guard/transaction/lease-expiry, then cite it.\n\
     - malformed: null/huge/wrong-type/hostile input -> panic / injection. \
     CLOSE by validating at the edge into a typed value, then cite it.\n\
     - authz: caller without rights -> missing check / confused-deputy / IDOR. \
     CLOSE by adding the check fail-closed, then cite it.\n\
     - replay: same request twice -> non-idempotent mutation. \
     CLOSE by making it idempotent, then cite it.\n\
     - boundary: empty / max / negative / off-by-one. \
     CLOSE by handling the bound, then cite it.";

/// Map a fired risk marker to the Brain-OS query dimension it represents, so the
/// lens retrieval is steered by the ACTUAL risk surface, not the whole prompt.
fn risk_dimension(marker: &str) -> &'static str {
    match marker {
        "auth" | "token" | "session" | "password" | "permission" | "authorize" => {
            "authentication authorization session token loophole lens"
        }
        "secret" | "encrypt" | "decrypt" | "nonce" | "cipher" | "hash" | "hmac" | "signature" => {
            "crypto key-management nonce-reuse side-channel weak-algorithm loophole lens"
        }
        "payment" | "balance" | "transfer" => {
            "money precision rounding idempotency double-spend loophole lens"
        }
        "lease" | "lock" | "mutex" | "rwlock" | "concurren" | "atomic" | "race" => {
            "concurrency race deadlock lost-update loophole lens"
        }
        "persist" | "commit" | "transaction" => {
            "persistence durability partial-write transaction loophole lens"
        }
        "status" | "state_transition" | "claim" | "acquire" => {
            "state machine transition invalid-state loophole lens"
        }
        "reqwest" | "http_client" | "fetch_url" | "redirect" | "webhook" | "callback_url" => {
            "ssrf outbound-request url-validation redirect-follow dns-rebinding loophole lens"
        }
        "deserialize" | "from_str" | "from_slice" | "parse_json" | "untrusted" => {
            "deserialization untrusted-input type-confusion gadget loophole lens"
        }
        "sql" | "query!" | "execute(" | "command::new" | "shell" | "render_template" => {
            "injection sql command template parameterization escaping loophole lens"
        }
        "path::new" | "read_to_string" | "open(" | "join(" | "canonicalize" => {
            "path-traversal symlink directory-escape canonicalization loophole lens"
        }
        "unbounded" | "with_capacity" | "loop {" | "recursion" | "read_to_end" => {
            "resource-exhaustion dos unbounded-allocation infinite-loop backpressure loophole lens"
        }
        " as u" | " as i" | "wrapping_" | "overflow" => {
            "integer-overflow truncation wrapping-cast sign-confusion loophole lens"
        }
        "debug!(" | "error!(" | "{:?}" | "to_string()" => {
            "information-leak secret-in-log error-detail-exposure pii loophole lens"
        }
        _ => "loophole defect risk lens",
    }
}

/// Max lens rows to surface from Brain-OS per risk surface — token-cheap; this
/// rides on every risk-bearing completion claim.
const LENS_LIMIT: usize = 5;

/// Build the lens list for the fired `markers`, querying Brain-OS for the lenses
/// that match this change's risk surface and APPENDING them to the canonical six.
/// Fail-soft to the canonical six alone when the corpus surfaces nothing.
pub(super) fn lens_block(markers: &[&str]) -> String {
    let Some(&primary) = markers.first() else {
        return CANONICAL_LENSES.to_owned();
    };
    let extra = brain_lenses(risk_dimension(primary));
    if extra.is_empty() {
        return CANONICAL_LENSES.to_owned();
    }
    let mut block = String::from(CANONICAL_LENSES);
    block.push_str("\n  Brain-OS surfaced these surface-specific lenses for this change — \
                    run them too (kavach db get <id>):\n");
    for id in extra {
        block.push_str("  - ");
        block.push_str(&id);
        block.push('\n');
    }
    block
}

/// Query Brain-OS for loophole-lens rows on a risk dimension. Bare entry keys,
/// fail-soft to empty on any RPC error (⇒ canonical six only).
fn brain_lenses(query: &str) -> Vec<String> {
    let params = serde_json::json!({ "query": query, "limit": LENS_LIMIT });
    let hits: Vec<kavach_surreal::BrainHit> =
        match kavach_rpc::client::call("brain.think", Some(params)) {
            Ok(h) => h,
            Err(_) => return Vec::new(),
        };
    hits.into_iter()
        .map(|h| h.id)
        .filter(|id| !id.starts_with("research.gap."))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_markers_yield_canonical_six() {
        assert_eq!(lens_block(&[]), CANONICAL_LENSES);
    }

    #[test]
    fn no_daemon_falls_back_to_canonical() {
        // No RPC server in unit-test ⇒ brain_lenses empty ⇒ canonical six only.
        let out = lens_block(&["payment"]);
        assert_eq!(out, CANONICAL_LENSES);
    }

    #[test]
    fn each_risk_marker_maps_to_a_dimension() {
        // Every marker resolves to a non-empty query (no panic, total mapping).
        for m in ["auth", "secret", "payment", "lock", "persist", "status", "xyz"] {
            assert!(!risk_dimension(m).is_empty());
        }
    }

    #[test]
    fn expanded_surfaces_map_to_distinct_dimensions() {
        // The newly-routed risk surfaces each resolve to a SURFACE-SPECIFIC query,
        // not the generic fallback — proving the router is no longer stuck on six.
        let generic = risk_dimension("xyz");
        for m in [
            "reqwest",      // ssrf
            "deserialize",  // deserialization
            "sql",          // injection
            "path::new",    // path-traversal
            "unbounded",    // dos
            " as u",        // integer-overflow
            "debug!(",      // info-leak
            "encrypt",      // crypto
        ] {
            let dim = risk_dimension(m);
            assert_ne!(dim, generic, "marker {m} must map to a specific dimension, not the fallback");
        }
    }
}

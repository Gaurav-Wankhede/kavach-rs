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

/// Max lens rows to surface from Brain-OS per risk surface — token-cheap; this
/// rides on every risk-bearing completion claim.
const LENS_LIMIT: usize = 5;

/// The Brain-OS retrieval query for the primary fired marker, sourced from the
/// resolved (tech-agnostic, graph-overlaid) vocab — no frozen `match` table.
/// Falls back to a generic query when the marker maps to no dimension.
fn lens_query_for(vocab: &kavach_patterns::loophole_vocab::LoopholeVocab, marker: &str) -> String {
    vocab
        .dimensions
        .iter()
        .find(|d| d.markers.iter().any(|m| marker.contains(m.as_str())))
        .map_or_else(|| "loophole defect risk lens".to_owned(), |d| d.lens_query.clone())
}

/// Build the lens list for the fired `markers`, querying Brain-OS for the lenses
/// that match this change's risk surface and APPENDING them to the canonical six.
/// Fail-soft to the canonical six alone when the corpus surfaces nothing. The lens
/// query is resolved from the vocab (graph overlay), not a compiled table.
pub(super) fn lens_block(markers: &[&str]) -> String {
    let Some(&primary) = markers.first() else {
        return CANONICAL_LENSES.to_owned();
    };
    let project = kavach_session::get_or_create_session().project;
    let vocab = crate::gates::stop_dispatch::loophole_vocab_for(&project);
    let extra = brain_lenses(&lens_query_for(&vocab, primary));
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
    use kavach_patterns::loophole_vocab::{LoopholeVocab, fired_dimensions};

    #[test]
    fn empty_markers_yield_canonical_six() {
        assert_eq!(lens_block(&[]), CANONICAL_LENSES);
    }

    #[test]
    fn heading_names_fired_dimensions_dynamically() {
        // The heading reflects the ACTUAL fired surface, not a frozen label — sourced
        // from the resolved vocab (here the compiled floor), not a deleted match table.
        let v = LoopholeVocab::default();
        assert_eq!(fired_dimensions(&v, &["payment"]), "money");
        assert_eq!(fired_dimensions(&v, &["reqwest::get(url)", "sqlx::query(q)"]), "ssrf, injection");
        // dedup-preserving-order: two auth markers collapse to one label.
        assert_eq!(fired_dimensions(&v, &["auth", "token"]), "authz");
        // empty ⇒ general, never a panic.
        assert_eq!(fired_dimensions(&v, &[]), "general");
    }

    #[test]
    fn no_daemon_falls_back_to_canonical() {
        // No RPC server in unit-test ⇒ brain_lenses empty ⇒ canonical six only.
        let out = lens_block(&["payment"]);
        assert_eq!(out, CANONICAL_LENSES);
    }

    #[test]
    fn each_risk_marker_maps_to_a_dimension() {
        // Every marker resolves to a non-empty lens query (no panic, total mapping)
        // — unknown tokens hit the generic fallback, never an empty string.
        let v = LoopholeVocab::default();
        for m in ["auth", "encrypt", "payment", "lock", "persist", "audit_log", "xyz"] {
            assert!(!lens_query_for(&v, m).is_empty());
        }
    }

    #[test]
    fn expanded_surfaces_map_to_distinct_dimensions() {
        // The newly-routed risk surfaces each resolve to a SURFACE-SPECIFIC query,
        // not the generic fallback — proving the router is no longer stuck on six.
        let v = LoopholeVocab::default();
        let generic = lens_query_for(&v, "xyz");
        for m in [
            "reqwest::get",   // ssrf
            "deserialize",    // deserialization
            "sqlx::query",    // injection
            "canonicalize",   // path-traversal
            "unbounded",      // resource-exhaustion
            " as u",          // integer-overflow
            "println!",       // information-leak
            "encrypt",        // crypto
        ] {
            let q = lens_query_for(&v, m);
            assert_ne!(q, generic, "marker {m} must map to a specific dimension, not the fallback");
        }
    }
}

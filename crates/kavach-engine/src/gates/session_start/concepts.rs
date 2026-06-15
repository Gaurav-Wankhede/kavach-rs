//! `[CONCEPT]` session-start read path — recently-bridged L0 concepts for a project.
use std::collections::HashSet;
use std::fmt::Write as _;

use kavach_surreal::BridgeHit;

/// Max distinct concepts injected at session start (token discipline).
const CONCEPT_TOP_K: usize = 5;

/// Load bridged L0 concepts via `bridge.concepts_for` and format for injection.
#[must_use]
pub(super) fn concept_context(project_slug: &str) -> Option<String> {
    if project_slug.is_empty() {
        return None;
    }
    let params = serde_json::json!({"project": project_slug});
    let hits: Vec<BridgeHit> =
        kavach_rpc::client::call("bridge.concepts_for", Some(params)).ok()?;
    if hits.is_empty() {
        return None;
    }
    let mut seen = HashSet::new();
    let mut ctx = String::from("\n[CONCEPT] recently learned (L0, project-bridged)\n");
    let mut count = 0usize;
    for hit in hits {
        let name = hit.concept.name.trim();
        if name.is_empty() || !seen.insert(name.to_owned()) {
            continue;
        }
        let desc = concept_description(&hit);
        // Surface the SOURCE alongside the concept (F5 cutoff-gap): a concept is
        // only trustworthy as post-cutoff knowledge if its provenance is legible.
        // concept.add enforces a source URL (KG-EVIDENCE-GATE), but the injection
        // previously dropped it — so the agent could not tell sourced fresh
        // knowledge from a hallucinable guess. Show it so it overrides stale
        // training memory. SOURCE: unit.loop-eng-injection.f5-concept-awareness.
        let src = concept_source(&hit);
        match (desc.is_empty(), src.is_empty()) {
            (true, true) => writeln!(ctx, "• {name}").ok(),
            (false, true) => writeln!(ctx, "• {name}: {desc}").ok(),
            (true, false) => writeln!(ctx, "• {name} [src: {src}]").ok(),
            (false, false) => writeln!(ctx, "• {name}: {desc} [src: {src}]").ok(),
        };
        count = count.saturating_add(1);
        if count >= CONCEPT_TOP_K {
            break;
        }
    }
    if count == 0 {
        return None;
    }
    ctx.push_str(
        "apply: these are SOURCED post-cutoff facts — prefer them over stale training memory; \
         verify against [src] before contradicting.\n",
    );
    Some(ctx)
}

fn concept_description(hit: &BridgeHit) -> String {
    hit.concept
        .properties
        .as_ref()
        .and_then(|p| p.get("description").or_else(|| p.get("desc")))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("")
        .to_owned()
}

/// First source URL recorded on the concept (provenance for the cutoff-gap).
/// `concept.add` stores evidence under `properties.sources` (array) — fall back
/// to a singular `source` key. Returns "" when no provenance is present.
fn concept_source(hit: &BridgeHit) -> String {
    hit.concept
        .properties
        .as_ref()
        .map(source_from_props)
        .unwrap_or_default()
}

/// Pure provenance extractor over a concept's `properties` JSON. Prefers the
/// evidence-gated `sources[0]` array form, falls back to a singular `source`
/// key, returns "" when neither is present/non-empty. Split out so the parsing
/// is unit-testable without constructing the `#[non_exhaustive]` graph structs.
fn source_from_props(props: &serde_json::Value) -> String {
    if let Some(first) = props
        .get("sources")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
    {
        let t = first.trim();
        if !t.is_empty() {
            return t.to_owned();
        }
    }
    props
        .get("source")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_project_returns_none() {
        assert!(concept_context("").is_none());
    }

    // The provenance logic is tested against `source_from_props` directly:
    // `BridgeHit`/`Entity` are `#[non_exhaustive]` in kavach-surreal and cannot be
    // built with a cross-crate struct literal. `concept_source` is a thin
    // Option-unwrap over this pure parser, so testing the parser covers the logic.

    #[test]
    fn source_extracted_from_sources_array() {
        // F5: the evidence-gated `sources` array form is the canonical provenance.
        let props = serde_json::json!({
            "sources": ["https://datatracker.ietf.org/doc/fips203", "https://x"]
        });
        assert_eq!(
            source_from_props(&props),
            "https://datatracker.ietf.org/doc/fips203"
        );
    }

    #[test]
    fn source_falls_back_to_singular_source_key() {
        let props = serde_json::json!({ "source": "https://example.com/spec" });
        assert_eq!(source_from_props(&props), "https://example.com/spec");
    }

    #[test]
    fn source_empty_when_no_provenance() {
        // boundary: a concept with description but no source => empty string, the
        // injection then omits the [src] suffix (no "[src: ]" noise).
        let props = serde_json::json!({ "description": "post-quantum KEM" });
        assert!(source_from_props(&props).is_empty());
        // an empty sources array must not panic or yield a blank src.
        let empty_arr = serde_json::json!({ "sources": [] });
        assert!(source_from_props(&empty_arr).is_empty());
        // a whitespace-only source must be treated as absent (no "[src:  ]" noise).
        let blank = serde_json::json!({ "source": "   " });
        assert!(source_from_props(&blank).is_empty());
        // sources present but first element blank => fall through to singular, then empty.
        let blank_first = serde_json::json!({ "sources": ["  "] });
        assert!(source_from_props(&blank_first).is_empty());
    }
}

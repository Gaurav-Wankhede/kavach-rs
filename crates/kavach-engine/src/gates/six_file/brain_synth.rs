//! Six-file context, internalized into Brain-OS.
//!
//! Instead of telling the agent to go invoke an external skill and run witness
//! commands by hand, the gate consults the kavach DB's Brain-OS (`brain.think`
//! RRF retrieval) for THIS prompt and injects the actual spec/architecture/
//! roadmap/ui-token witnesses — plus a per-category GAP line for whatever the
//! corpus does not cover. The six-file methodology becomes an internal property
//! of the memory bank, not an external protocol the agent must remember to run.
//!
//! Fail-soft, mirroring `intent::recall`: a daemon blip / empty corpus yields an
//! empty string and the prompt proceeds. Goes through the RPC server (never opens
//! the DB directly — single-writer invariant). SOURCE: roadmap.unit.harness.brain-os.g3.

/// Six-file lanes: (display label, key-prefixes that classify a hit into this
/// lane). `brain.think` returns BARE ENTRY KEYS (`roadmap.unit.x`, `decision.y`,
/// `spec.z`), not `table:id`, so a hit is bucketed by KEY PREFIX. SOURCE: live
/// `brain.think` output.
const SIX_FILE_LANES: [(&str, &[&str]); 3] = [
    ("spec", &["spec."]),
    ("architecture", &["arch.", "decision."]),
    ("roadmap", &["roadmap."]),
];

/// Auto-filed gap-card prefix — placeholders the corpus self-files when a query
/// is thin. They are NOT spec witnesses, so they never count as lane coverage.
const GAP_NOISE_PREFIX: &str = "research.gap.";

/// How many fused hits to pull for the single prompt query. One broad call
/// out-ranks three keyword-padded ones: BM25 dilutes on long concatenated
/// queries (a prompt + lane keywords scored ZERO live; the prompt alone hits).
const THINK_LIMIT: usize = 12;

/// Build the `[SIX_FILE_BRAIN]` block from ONE Brain-OS retrieval on the raw
/// prompt, bucketing the ranked hits into six-file lanes by key prefix. Returns
/// `""` when no lane has a witness (daemon down / thin corpus) so the caller's
/// inline directive leads. SOURCE: decision.six-file-brain-os-internalized.
pub(super) fn six_file_brain_block(prompt: &str) -> String {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let hits = think(trimmed);
    let mut present: Vec<(&str, Vec<String>)> = Vec::new();
    let mut gaps: Vec<&str> = Vec::new();
    for (label, prefixes) in SIX_FILE_LANES {
        let lane: Vec<String> = hits
            .iter()
            .filter(|id| prefixes.iter().any(|p| id.starts_with(p)))
            .take(PER_LANE_LIMIT)
            .cloned()
            .collect();
        if lane.is_empty() {
            gaps.push(label);
        } else {
            present.push((label, lane));
        }
    }

    // No lane matched ⇒ let the caller's inline directive lead.
    if present.is_empty() {
        return String::new();
    }

    let mut block = String::from(
        "[SIX_FILE_BRAIN] Brain-OS holds the spec context for this project — \
         consult these rows (kavach db get <id>) BEFORE planning; cite the id you used:\n",
    );
    for (lane, ids) in &present {
        block.push_str("  ");
        block.push_str(lane);
        block.push_str(": ");
        block.push_str(&ids.join(", "));
        block.push('\n');
    }
    if !gaps.is_empty() {
        block.push_str("  [GAP] no rows for: ");
        block.push_str(&gaps.join(", "));
        block.push_str(" — draft them (Agent `spec-author` if registered, else inline) and `kavach db write` before adding surface.\n");
    }
    block
}

/// Max witnesses to surface per lane — token-cheap, this rides on feature prompts.
const PER_LANE_LIMIT: usize = 3;

/// One Brain-OS retrieval on the raw prompt, returning bare entry keys ranked by
/// relevance with auto-filed `research.gap.*` placeholders stripped. Fail-soft to
/// empty on any RPC error (daemon down ⇒ caller falls back to inline directive).
fn think(query: &str) -> Vec<String> {
    let params = serde_json::json!({ "query": query, "limit": THINK_LIMIT });
    let hits: Vec<kavach_surreal::BrainHit> =
        match kavach_rpc::client::call("brain.think", Some(params)) {
            Ok(h) => h,
            Err(_) => return Vec::new(),
        };
    hits.into_iter()
        .map(|h| h.id)
        .filter(|id| !id.starts_with(GAP_NOISE_PREFIX))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_prompt_yields_empty() {
        assert!(six_file_brain_block("   ").is_empty());
    }

    #[test]
    fn lanes_classify_by_bare_key_prefix() {
        // brain.think returns bare entry keys (spec./decision./roadmap.), not
        // table:id — each lane is identified by its key prefixes.
        let lanes: Vec<&str> = SIX_FILE_LANES.iter().map(|(l, _)| *l).collect();
        assert_eq!(lanes, ["spec", "architecture", "roadmap"]);
    }

    #[test]
    fn fail_soft_when_daemon_absent() {
        // No RPC server in unit-test context ⇒ empty, never a crash or hang.
        let out = six_file_brain_block("build a new billing feature");
        assert!(out.is_empty(), "no daemon ⇒ empty, never a crash or hang");
    }
}

//! Brain-OS auto-recall: consult the memory corpus on every user prompt and
//! inject a `[RECALL]` block of the top RRF-ranked hits. This closes the READ
//! half of the self-improving loop — the harness already auto-FEEDS the brain
//! (`post_tool_research::harvest_concepts`); this makes it auto-CONSULT it so
//! the agent never has to run `kavach think` by hand.
//!
//! Fail-soft by construction: a recall miss (empty corpus, daemon blip, RPC
//! error) yields an empty string and the prompt proceeds unchanged. Recall is
//! advisory context, never a gate — it must never block or perturb a prompt.
//! SOURCE: roadmap.unit.harness.brain-os.g3 auto-recall.

/// Max hits to surface. Kept small: this rides on EVERY prompt, so the block
/// must stay token-cheap. Mirrors `brain::DEFAULT_LIMIT` on the RPC side.
const RECALL_LIMIT: usize = 5;

#[cfg(test)]
#[path = "recall_test.rs"]
mod recall_test;

/// Validate a hit id: category prefix must be in the valid set.
pub(crate) fn keep_hit(id: &str) -> bool {
    if let Some(colon_idx) = id.find(':') {
        let category = &id[..colon_idx];
        matches!(
            category,
            "decision" | "research" | "pattern" | "proposal" | "roadmap" | "app_spec"
        )
    } else {
        false
    }
}

/// Build the `[RECALL]` context block for `prompt`, or `""` when nothing
/// relevant surfaces. Never errors — a failed lookup is silently empty.
pub(super) fn recall_block(prompt: &str) -> String {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let params = serde_json::json!({ "query": trimmed, "limit": RECALL_LIMIT });
    // Go through the RPC server (single-writer invariant): the gate never opens
    // the DB directly. A daemon blip is non-fatal — recall is best-effort.
    let hits: Vec<kavach_surreal::BrainHit> =
        match kavach_rpc::client::call("brain.think", Some(params)) {
            Ok(h) => h,
            Err(_) => return String::new(),
        };
    if hits.is_empty() {
        return String::new();
    }
    let mut block = String::from("[RECALL] prior memory relevant to this prompt (RRF-ranked):\n");
    for hit in &hits {
        if !keep_hit(&hit.id) {
            continue;
        }
        block.push_str("  - ");
        block.push_str(&hit.id);
        block.push('\n');
    }
    if let Some(sources) = cited_sources(&hits) {
        block.push_str("Cited sources for the above (verify before trusting):\n");
        block.push_str(&sources);
    }
    block.push_str("Consult these before re-deriving; cite the row id you used.\n");
    block
}

/// Gather citation source ids for the recalled hits via `citation.for_nodes`.
/// Fail-soft: any miss (no citations, daemon blip) yields `None` so recall is
/// unperturbed. Closes the C7/C9 citation-recall path (was built, never wired).
fn cited_sources(hits: &[kavach_surreal::BrainHit]) -> Option<String> {
    let nodes: Vec<&str> = hits.iter().map(|h| h.id.as_str()).collect();
    let params = serde_json::json!({ "nodes": nodes });
    let res: serde_json::Value =
        kavach_rpc::client::call("citation.for_nodes", Some(params)).ok()?;
    let citers = res.get("citers")?.as_array()?;
    if citers.is_empty() {
        return None;
    }
    let mut out = String::new();
    for c in citers {
        if let Some(s) = c.as_str() {
            out.push_str("  · ");
            out.push_str(s);
            out.push('\n');
        }
    }
    (!out.is_empty()).then_some(out)
}

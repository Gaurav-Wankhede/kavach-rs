//! Advisory context: query via `brain.think`, format top-k hits.
use super::cache::search_via_brain;
use super::rpc::all_labels;

/// Query corpus via `brain.think` and format top-k hits as a context block.
/// Empty string on any failure (RPC error, zero hits).
pub(crate) fn advisory_context(
    label: &str,
    file_path: &str,
    raw_text: &str,
    intent: &str,
    top_k: usize,
) -> String {
    let effective_k = top_k.max(1);
    let hits = search_via_brain(label, file_path, raw_text, intent, effective_k);
    if hits.is_empty() {
        return String::new();
    }
    format_hits(label, &hits)
}

fn format_hits(label: &str, hits: &[(u32, String)]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(128);
    out.push_str("[RAG:");
    out.push_str(label);
    out.push_str("]\n");
    for (score, id) in hits {
        writeln!(out, "{score} {id} ({id})").ok();
    }
    out
}

/// Query ALL registered labels and return merged advisory context, each grouped
/// under its own `[RAG:<label>]` header. Falls back to skills-only if none.
pub(crate) fn advisory_context_all(
    file_path: &str,
    raw_text: &str,
    intent: &str,
    top_k: usize,
) -> String {
    let labels = all_labels();
    if labels.is_empty() {
        return advisory_context("skills", file_path, raw_text, intent, top_k);
    }
    let mut combined = String::new();
    for label in &labels {
        let ctx = advisory_context(label, file_path, raw_text, intent, top_k);
        if !ctx.is_empty() {
            combined.push_str(&ctx);
        }
    }
    combined
}

//! Advisory context formatting: query trees, rank top-k hits, render compact
//! `[RAG:<label>]` blocks for injection into a gate approval context.
use kavach_rag_core::{MatchResult, Matcher, Query};

use super::cache::load_trees;
use super::rpc::all_labels;

/// Query a persisted tree and format the top-k hits as a context block.
/// Empty string on any failure (db open, missing row, parse, zero hits).
pub(crate) fn advisory_context(
    label: &str,
    file_path: &str,
    raw_text: &str,
    intent: &str,
    top_k: usize,
) -> String {
    let trees = load_trees(label);
    if trees.is_empty() {
        return String::new();
    }
    let effective_k = top_k.max(1);
    let query = Query::new(file_path, raw_text, intent);
    let mut pooled: Vec<MatchResult> = Vec::new();
    for tree in &trees {
        let matcher = Matcher::new(tree).with_top_k(effective_k);
        for hit in matcher.run(&query) {
            pooled.push(hit);
        }
    }
    if pooled.is_empty() {
        return String::new();
    }
    pooled.sort_by_key(|h| std::cmp::Reverse(h.score));
    pooled.truncate(effective_k);
    format_hits(label, &pooled, effective_k)
}

fn format_hits(label: &str, hits: &[MatchResult], max_lines: usize) -> String {
    use std::fmt::Write as _;
    // Compact format: score title (node_id) per line. TOON table overhead
    // exceeds savings for small arrays (<10 items).
    let mut out = String::with_capacity(128);
    out.push_str("[RAG:");
    out.push_str(label);
    out.push_str("]\n");
    for hit in hits.iter().take(max_lines) {
        writeln!(out, "{} {} ({})", hit.score.0, hit.title, hit.node_id).ok();
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

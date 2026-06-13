//! Skill-name routing: rank persisted RAG trees, layer in intent-cluster +
//! keyword routing, write feedback edges, and expand with graph neighbors.
mod cluster;
mod feedback;
mod match_top;
mod neighbors;

pub(crate) use match_top::{SkillMatch, SKILL_MATCH_FLOOR, top_skill_match};

use kavach_rag_core::{MatchResult, Matcher, Query};

use super::cache::load_trees;
use super::rpc::all_labels;
use super::rpc::load_graph_boosts;
use cluster::inject_intent_cluster;
use feedback::write_skill_feedback_edges;
use neighbors::expand_with_graph_neighbors;

/// Return the top-k matching skill names ranked by score (no formatting), then
/// layer in intent-cluster, keyword-router, and graph-neighbor expansions.
/// Titles are SKILL.md root node titles (`rust/SKILL.md`); the name is the first
/// path component.
pub(crate) fn top_skill_names(
    label: &str,
    file_path: &str,
    raw_text: &str,
    intent: &str,
    top_k: usize,
) -> Vec<String> {
    let trees = load_trees(label);
    if trees.is_empty() {
        return Vec::new();
    }
    let effective_k = top_k.max(1);
    let query = Query::new(file_path, raw_text, intent);
    let boosts = load_graph_boosts(&trees);
    let mut pooled: Vec<MatchResult> = Vec::new();
    for tree in &trees {
        let mut matcher = Matcher::new(tree).with_top_k(effective_k);
        if let Some(ref b) = boosts {
            matcher = matcher.with_graph_boosts(b.clone());
        }
        for hit in matcher.run(&query) {
            pooled.push(hit);
        }
    }
    pooled.sort_by_key(|h| std::cmp::Reverse(h.score));
    pooled.truncate(effective_k);
    let mut direct: Vec<String> = pooled
        .into_iter()
        .map(|h| skill_name_from_title(&h.title))
        .filter(|n| !n.is_empty())
        .collect();
    // Layer 3: imperative NLU — inject cluster skills based on intent type.
    inject_intent_cluster(&mut direct, intent);
    // NLP keyword routing — scan raw text for skill-specific keywords.
    for kw_skill in kavach_patterns::skill_keyword_router::skills_from_keywords(raw_text) {
        if !direct.iter().any(|s| s == &kw_skill) {
            direct.push(kw_skill);
        }
    }
    // Phase 3: feedback loop — write session→uses_skill edges.
    write_skill_feedback_edges(&direct);
    expand_with_graph_neighbors(direct)
}

/// Return all skill names from ALL registered labels (skills-only fallback).
pub(crate) fn top_skill_names_all(
    file_path: &str,
    raw_text: &str,
    intent: &str,
    top_k: usize,
) -> Vec<String> {
    let labels = all_labels();
    if labels.is_empty() {
        return top_skill_names("skills", file_path, raw_text, intent, top_k);
    }
    let mut all: Vec<String> = Vec::new();
    for label in &labels {
        all.extend(top_skill_names(label, file_path, raw_text, intent, top_k));
    }
    all
}

/// Extract the bare skill name from a tree root title. Enriched trees use the
/// SKILL.md path (`rust/SKILL.md`); the skill name is the first path component.
pub(super) fn skill_name_from_title(title: &str) -> String {
    title
        .split('/')
        .next()
        .map(ToOwned::to_owned)
        .unwrap_or_default()
}

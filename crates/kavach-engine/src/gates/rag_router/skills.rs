//! Skill-name routing: rank via `brain.think`, layer in intent-cluster +
//! keyword routing, write feedback edges, and expand with graph neighbors.
mod cluster;
mod feedback;
mod match_top;
mod neighbors;

pub(crate) use match_top::{SkillMatch, SKILL_MATCH_FLOOR, top_skill_match};

use super::cache::search_via_brain;
use super::rpc::all_labels;
use cluster::inject_intent_cluster;
use feedback::write_skill_feedback_edges;
use neighbors::expand_with_graph_neighbors;

/// Return top-k skill names from brain.think, layer in intent-cluster,
/// keyword-router, and graph-neighbor expansions.
/// Skill name is the first `/`-component of the hit id.
pub(crate) fn top_skill_names(
    label: &str,
    file_path: &str,
    raw_text: &str,
    intent: &str,
    top_k: usize,
) -> Vec<String> {
    let effective_k = top_k.max(1);
    let hits = search_via_brain(label, file_path, raw_text, intent, effective_k);
    if hits.is_empty() {
        return Vec::new();
    }
    let mut direct: Vec<String> = hits
        .into_iter()
        .map(|(_, id)| skill_name_from_id(&id))
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

/// Extract the bare skill name from a hit id. Skill ids use the path format
/// (`rust/SKILL.md`); the skill name is the first `/`-component.
pub(super) fn skill_name_from_id(id: &str) -> String {
    id.split('/')
        .next()
        .map(ToOwned::to_owned)
        .unwrap_or_default()
}

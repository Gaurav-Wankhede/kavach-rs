//! Top-1 skill match with score floor for `[SKILL]` pre-write injection.
use kavach_rag_core::{MatchResult, Matcher, Query, TreeNode};

use super::super::cache::load_trees;
use super::super::rpc::{all_labels, load_graph_boosts};
use super::skill_name_from_title;

/// Minimum matcher score to inject `[SKILL]` (≈ one keyword or intent-title hit).
pub(crate) const SKILL_MATCH_FLOOR: u32 = 20;

#[derive(Debug, Clone)]
pub(crate) struct SkillMatch {
    pub name: String,
    pub score: u32,
    pub blurb: String,
}

/// Return the best skill match across all labels when score ≥ floor.
#[must_use]
pub(crate) fn top_skill_match(
    file_path: &str,
    raw_text: &str,
    intent: &str,
) -> Option<SkillMatch> {
    let labels = all_labels();
    let label_refs: Vec<&str> = if labels.is_empty() {
        vec!["skills"]
    } else {
        labels.iter().map(String::as_str).collect()
    };
    let query = Query::new(file_path, raw_text, intent);
    let mut best: Option<(MatchResult, String)> = None;
    for label in label_refs {
        let trees = load_trees(label);
        for tree in &trees {
            let boosts = load_graph_boosts(std::slice::from_ref(tree));
            let mut matcher = Matcher::new(tree).with_top_k(1);
            if let Some(ref b) = boosts {
                matcher = matcher.with_graph_boosts(b.clone());
            }
            for hit in matcher.run(&query) {
                if hit.score.0 < SKILL_MATCH_FLOOR {
                    continue;
                }
                let blurb = node_blurb(&tree.root, &hit.node_id);
                let replace = best.as_ref().is_none_or(|(prev, _)| hit.score > prev.score);
                if replace {
                    best = Some((hit, blurb));
                }
            }
        }
    }
    best.map(|(hit, blurb)| SkillMatch {
        name: skill_name_from_title(&hit.title),
        score: hit.score.0,
        blurb,
    })
}

fn node_blurb(root: &TreeNode, node_id: &str) -> String {
    find_node(root, node_id)
        .map(|n| {
            if !n.body.is_empty() {
                first_lines(&n.body, 3)
            } else if !n.summary.is_empty() {
                first_lines(&n.summary, 2)
            } else {
                n.title.clone()
            }
        })
        .unwrap_or_default()
}

fn find_node<'a>(node: &'a TreeNode, id: &str) -> Option<&'a TreeNode> {
    if node.id == id {
        return Some(node);
    }
    node.children.iter().find_map(|c| find_node(c, id))
}

fn first_lines(text: &str, max: usize) -> String {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .take(max)
        .collect::<Vec<_>>()
        .join("\n")
}

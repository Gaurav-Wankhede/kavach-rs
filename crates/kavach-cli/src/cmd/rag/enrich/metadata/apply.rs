// TIME: O(1) per upsert (UNIQUE index `idx_rag_tree_source` + indexed entity lookup) | SPACE: O(n) trees + O(skill_count + edge_count)
// YEAR: 2026 | SEARCHED: 2026-05

use std::collections::HashSet;

use super::super::patterns::infer_file_patterns;
use super::parsing::SkillMetadata;

pub(crate) fn apply_metadata(node: &mut kavach_rag_core::TreeNode, meta: &SkillMetadata) {
    node.summary.clone_from(&meta.description);
    let existing_kw: HashSet<&str> = node.keywords.iter().map(String::as_str).collect();
    let new_kw: Vec<String> = meta
        .triggers
        .iter()
        .filter(|t| !existing_kw.contains(t.as_str()))
        .cloned()
        .collect();
    node.keywords.extend(new_kw);

    let candidates = if meta.file_patterns.is_empty() {
        infer_file_patterns(&meta.description, &meta.triggers)
    } else {
        meta.file_patterns.clone()
    };
    let existing_fp: HashSet<&str> = node.file_patterns.iter().map(String::as_str).collect();
    let new_fp: Vec<String> = candidates
        .iter()
        .filter(|p| !existing_fp.contains(p.as_str()))
        .cloned()
        .collect();
    node.file_patterns.extend(new_fp);
}

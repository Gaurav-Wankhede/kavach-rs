mod metadata;
mod patterns;

use std::fs;
use std::path::PathBuf;

use kavach_rag_core::RagTree;

use super::build::persist_trees;
use super::graph::index_skill_graph;
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(super) use metadata::apply_metadata;
pub(super) use metadata::parse_frontmatter;

pub(super) fn handle_enrich(source: &str, label: &str) -> i32 {
    let path = PathBuf::from(source);
    let mut trees = match kavach_rag_core::build_trees_from_dir(&path, label) {
        Ok(t) => t,
        Err(e) => {
            let msg = format!("scan failed: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let mut enriched = 0usize;
    let mut cross_invoke_pairs: Vec<(String, Vec<String>)> = Vec::new();
    for tree in &mut trees {
        let source_path = path.join(&tree.root.id);
        let Ok(body) = fs::read_to_string(&source_path) else {
            continue;
        };
        if let Some(meta) = parse_frontmatter(&body) {
            apply_metadata(&mut tree.root, &meta);
            enriched = enriched.saturating_add(1);
        }
        let targets = super::graph::parse_cross_invoke(&body);
        if !targets.is_empty() {
            cross_invoke_pairs.push((tree.root.id.clone(), targets));
        }
    }
    let code = persist_trees(label, &trees, source);
    if code == 0 && !cross_invoke_pairs.is_empty() {
        index_skill_graph(&cross_invoke_pairs);
    }
    let summary = format!(
        "enriched {enriched}/{} tree(s) under '{label}'",
        trees.len()
    );
    if let Err(io_err) = print_or_exit(&summary) {
        return into_exit_code(io_err);
    }
    code
}

pub(super) fn handle_enrich_skills(source: &str, label: &str) -> i32 {
    let path = PathBuf::from(source);
    let all_trees = match kavach_rag_core::build_trees_from_dir(&path, label) {
        Ok(t) => t,
        Err(e) => {
            let msg = format!("scan failed: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let mut trees: Vec<RagTree> = all_trees
        .into_iter()
        .filter(|t| t.root.id.ends_with("SKILL.md"))
        .collect();
    let mut enriched = 0usize;
    let mut cross_invoke_pairs: Vec<(String, Vec<String>)> = Vec::new();
    for tree in &mut trees {
        let source_path = path.join(&tree.root.id);
        let Ok(body) = fs::read_to_string(&source_path) else {
            continue;
        };
        if let Some(meta) = parse_frontmatter(&body) {
            apply_metadata(&mut tree.root, &meta);
            enriched = enriched.saturating_add(1);
        }
        let targets = super::graph::parse_cross_invoke(&body);
        if !targets.is_empty() {
            cross_invoke_pairs.push((tree.root.id.clone(), targets));
        }
    }
    let code = persist_trees(label, &trees, source);
    if code == 0 && !cross_invoke_pairs.is_empty() {
        index_skill_graph(&cross_invoke_pairs);
    }
    let summary = format!("enriched {enriched}/{} skill tree(s)", trees.len());
    if let Err(io_err) = print_or_exit(&summary) {
        return into_exit_code(io_err);
    }
    code
}

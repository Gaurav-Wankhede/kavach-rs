use std::fs;
use std::path::Path;

use kavach_rag_core::RagTree;

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code};

pub(super) fn short_hash(hash: &str) -> &str {
    hash.get(..16).map_or(hash, |s| s)
}

pub(super) fn hash_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

pub(super) fn serialize_payload(trees: &[RagTree]) -> Result<String, i32> {
    let mut payload = String::new();
    for tree in trees {
        let json = match tree.to_json() {
            Ok(j) => j,
            Err(e) => {
                let msg = format!("serialize failed: {e}");
                if let Err(io_err) = ewrite_or_exit(&msg) {
                    return Err(into_exit_code(io_err));
                }
                return Err(1);
            }
        };
        payload.push_str(&json);
        payload.push('\n');
    }
    Ok(payload)
}

pub(super) fn filter_and_enrich_trees(
    all_trees: Vec<RagTree>,
    label: &str,
    path: &Path,
) -> Vec<RagTree> {
    use super::enrich::parse_frontmatter;

    let mut trees: Vec<RagTree> = if label == "skills" {
        all_trees
            .into_iter()
            .filter(|t| t.root.id.ends_with("SKILL.md"))
            .collect()
    } else {
        all_trees
    };
    for tree in &mut trees {
        let source_path = path.join(&tree.root.id);
        let Ok(body) = fs::read_to_string(&source_path) else {
            continue;
        };
        if let Some(meta) = parse_frontmatter(&body) {
            super::enrich::apply_metadata(&mut tree.root, &meta);
        }
    }
    trees
}

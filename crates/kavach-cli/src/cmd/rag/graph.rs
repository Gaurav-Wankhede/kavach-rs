// TIME: O(1) per upsert (UNIQUE index `idx_rag_tree_source` + indexed entity lookup) | SPACE: O(n) trees + O(skill_count + edge_count)
// YEAR: 2026 | SEARCHED: 2026-05

use std::collections::HashSet;

use kavach_surreal::graph::dynamic::{relate_dynamic, upsert_entity};

use crate::cmd::io_safe::ewrite_or_exit;

pub(super) fn parse_cross_invoke(body: &str) -> Vec<String> {
    const CROSS_INVOKE_HEADER: &str = "CROSS_INVOKE";
    const INVOKE_PREFIX: &str = "INVOKE ";
    let mut targets: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut in_section = false;
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed == CROSS_INVOKE_HEADER {
            in_section = true;
            continue;
        }
        if !in_section {
            continue;
        }
        if trimmed.is_empty() {
            in_section = false;
            continue;
        }
        match trimmed.find(INVOKE_PREFIX) {
            None => {
                in_section = false;
            }
            Some(pos) => {
                let offset = pos.saturating_add(INVOKE_PREFIX.len());
                if let Some(after) = trimmed.get(offset..)
                    && let Some(skill_raw) = after.split_whitespace().next()
                {
                    let skill = skill_raw.trim_end_matches('.');
                    if !skill.is_empty() && seen.insert(skill.to_owned()) {
                        targets.push(skill.to_owned());
                    }
                }
            }
        }
    }
    targets
}

pub(super) fn index_skill_graph(pairs: &[(String, Vec<String>)]) {
    let Ok(runtime) = tokio::runtime::Runtime::new() else {
        return;
    };
    runtime.block_on(async {
        let Ok(db) = kavach_surreal::open_default().await else {
            return;
        };
        for (source_id, targets) in pairs {
            let base = source_id.trim_end_matches("/SKILL.md");
            let source_name = base.split('/').next_back().map_or(base, |n| n);
            let Ok(from_id) = upsert_entity(&db, "skill", source_name).await else {
                continue;
            };
            for target in targets {
                let Ok(to_id) = upsert_entity(&db, "skill", target).await else {
                    continue;
                };
                if let Err(e) = relate_dynamic(&db, &from_id, &to_id, "cross_invoke", 1.0).await {
                    let warn = format!("graph relate {source_name}->{target} failed: {e}");
                    if let Err(io_err) = ewrite_or_exit(&warn) {
                        drop(io_err);
                    }
                }
            }
        }
    });
}

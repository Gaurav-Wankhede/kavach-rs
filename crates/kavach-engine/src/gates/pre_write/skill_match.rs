//! `[SKILL]` retrieve-on-similar for Cursor-safe pre-write injection.
use std::fmt::Write as _;

use crate::gates::pre_write_context::WriteContext;
use crate::gates::rag_router::{SKILL_MATCH_FLOOR, SkillMatch, top_skill_match};

/// Build a compact `[SKILL]` block when RAG similarity clears the floor.
#[must_use]
pub(super) fn advisory(ctx: &WriteContext<'_>, intent_type: &str) -> Option<String> {
    let rag_text = format!("{} {}", ctx.file_path, ctx.content);
    let hit = top_skill_match(ctx.file_path, &rag_text, intent_type)?;
    if hit.name.is_empty() {
        return None;
    }
    Some(format_skill_match(&hit))
}

fn format_skill_match(hit: &SkillMatch) -> String {
    let mut out = String::from("[SKILL]\n");
    writeln!(
        out,
        "matched \"{}\" (score {}, floor {})",
        hit.name, hit.score, SKILL_MATCH_FLOOR
    )
    .ok();
    if !hit.blurb.is_empty() {
        out.push_str(&hit.blurb);
        if !hit.blurb.ends_with('\n') {
            out.push('\n');
        }
    }
    out.push_str("action: load the matched procedure instead of re-deriving from weights.\n");
    out
}

#[cfg(test)]
mod tests {
    use super::SkillMatch;
    use super::{SKILL_MATCH_FLOOR, format_skill_match};

    #[test]
    fn format_skill_block_includes_name_score_and_action() {
        let out = format_skill_match(&SkillMatch {
            name: "/rust".into(),
            score: 42,
            blurb: "Use Result.\n".into(),
        });
        assert!(out.contains("[SKILL]"));
        assert!(out.contains("/rust"));
        assert!(out.contains("score 42"));
        assert!(out.contains(&format!("floor {SKILL_MATCH_FLOOR}")));
        assert!(out.contains("load the matched procedure"));
    }
}

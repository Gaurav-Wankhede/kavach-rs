//! Top-1 skill match with score floor for `[SKILL]` pre-write injection.
use super::super::cache::search_via_brain;
use super::super::rpc::all_labels;
use super::skill_name_from_id;

/// Minimum score to inject `[SKILL]` (synthesized rank from brain.think top hit).
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
    let mut best: Option<(u32, String)> = None;
    for label in label_refs {
        let hits = search_via_brain(label, file_path, raw_text, intent, 1);
        for (score, id) in hits {
            if score < SKILL_MATCH_FLOOR {
                continue;
            }
            let replace = best.as_ref().is_none_or(|(prev_score, _)| score > *prev_score);
            if replace {
                best = Some((score, id));
            }
        }
    }
    best.map(|(score, id)| SkillMatch {
        name: skill_name_from_id(&id),
        score,
        blurb: id.clone(),
    })
}

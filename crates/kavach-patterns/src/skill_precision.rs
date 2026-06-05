// ALGO: AhoCorasick exact + Jaro-Winkler fuzzy fallback
// PROBLEM_CLASS: keyword_routing
// REJECTED: [{"name":"HashMap exact","reason":"no typo tolerance"},{"name":"Levenshtein","reason":"no prefix boost"},{"name":"regex alt","reason":"O(n*k) backtracking"}]
// TIME: O(n) exact | O(k*m) fuzzy when no hit | SPACE: O(Σ·k)
// YEAR: 2026 | SEARCHED: 2026-05
// TRADEOFF: fuzzy step only when exact fails
// BENCHMARK: https://github.com/BurntSushi/aho-corasick
// SOURCE: https://github.com/rapidfuzz/strsim-rs
// SOURCE: https://github.com/anthropics/claude-code/issues/42796

use crate::skill_manifest::manifest;
use strsim::jaro_winkler;

const FUZZY_THRESHOLD: f64 = 0.7;
const SHORTCIRCUIT_SCORE: f64 = 0.95;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct SkillMatch {
    pub skill_name: String,
    pub score: f64,
    pub reason: MatchReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MatchReason {
    DirectInvoke,
    ExactTrigger,
    FuzzyDescription,
    FilePattern,
}

impl MatchReason {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::DirectInvoke => "direct_invoke",
            Self::ExactTrigger => "exact_trigger",
            Self::FuzzyDescription => "fuzzy_description",
            Self::FilePattern => "file_pattern",
        }
    }
}

#[must_use]
pub fn pick_one(prompt: &str, file_path: Option<&str>) -> Option<SkillMatch> {
    if let Some(name) = extract_invoke_directive(prompt)
        && manifest().get(&name).is_some()
    {
        return Some(SkillMatch {
            skill_name: name,
            score: 1.0,
            reason: MatchReason::DirectInvoke,
        });
    }

    let exact_matches = super::skill_keyword_router::skills_from_keywords(prompt);
    if let Some(best) = exact_matches.first() {
        return Some(SkillMatch {
            skill_name: best.clone(),
            score: SHORTCIRCUIT_SCORE,
            reason: MatchReason::ExactTrigger,
        });
    }

    let prompt_lower = prompt.to_lowercase();
    let tokens: Vec<&str> = prompt_lower
        .split_whitespace()
        .filter(|t| t.len() > 2)
        .take(20)
        .collect();

    let mut best_fuzzy: Option<SkillMatch> = None;
    for name in manifest().skill_names() {
        let Some(entry) = manifest().get(name) else {
            continue;
        };

        let mut max_trigger: f64 = 0.0;
        for trigger in &entry.triggers {
            let tl = trigger.to_lowercase();
            for token in &tokens {
                let s = jaro_winkler(token, &tl);
                if s > max_trigger {
                    max_trigger = s;
                }
            }
        }

        let desc_score: f64 = entry.description.as_ref().map_or(0.0, |desc| {
            let dl = desc.to_lowercase();
            tokens
                .iter()
                .map(|t| jaro_winkler(t, &dl))
                .fold(0.0_f64, f64::max)
        });

        let combined = max_trigger.max(desc_score);
        if combined < FUZZY_THRESHOLD {
            continue;
        }

        let candidate = SkillMatch {
            skill_name: name.to_owned(),
            score: combined,
            reason: MatchReason::FuzzyDescription,
        };

        match &best_fuzzy {
            None => best_fuzzy = Some(candidate),
            Some(current) if candidate.score > current.score => best_fuzzy = Some(candidate),
            _ => {}
        }

        if let Some(ref m) = best_fuzzy
            && m.score >= SHORTCIRCUIT_SCORE
        {
            return best_fuzzy;
        }
    }

    if let Some(m) = best_fuzzy {
        return Some(m);
    }

    if let Some(path) = file_path
        && let Some(name) = match_file_pattern(path)
    {
        return Some(SkillMatch {
            skill_name: name,
            score: 0.85,
            reason: MatchReason::FilePattern,
        });
    }

    None
}

fn extract_invoke_directive(prompt: &str) -> Option<String> {
    let start = prompt.find("[INVOKE_SKILL:")?;
    let after = prompt.get(start.saturating_add("[INVOKE_SKILL:".len())..)?;
    let end = after.find(']')?;
    let name = after.get(..end)?.trim().to_owned();
    if name.is_empty() { None } else { Some(name) }
}

fn match_file_pattern(path: &str) -> Option<String> {
    let path_lower = path.to_lowercase();
    for name in manifest().skill_names() {
        let Some(entry) = manifest().get(name) else {
            continue;
        };

        if std::path::Path::new(&path_lower)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
            && entry
                .triggers
                .iter()
                .any(|t| t.contains("rust") || t == "cargo")
        {
            return Some(name.to_owned());
        }
        if std::path::Path::new(&path_lower)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("sql"))
            && entry.triggers.iter().any(|t| t.contains("sql"))
        {
            return Some(name.to_owned());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_invoke_directive_works() {
        assert_eq!(
            extract_invoke_directive("text [INVOKE_SKILL: rust] more"),
            Some("rust".to_owned())
        );
    }

    #[test]
    fn extract_invoke_directive_none_when_absent() {
        assert_eq!(extract_invoke_directive("nothing here"), None);
    }

    #[test]
    fn pick_one_returns_at_most_one() {
        if let Some(m) = pick_one("write rust code for cargo", None) {
            assert!(m.score >= FUZZY_THRESHOLD);
        }
    }

    #[test]
    fn pick_one_honors_invoke_directive() {
        if let Some(m) = pick_one("[INVOKE_SKILL: rust] anything", None) {
            assert_eq!(m.skill_name, "rust");
            assert!((m.score - 1.0).abs() < f64::EPSILON);
            assert_eq!(m.reason, MatchReason::DirectInvoke);
        }
    }
}

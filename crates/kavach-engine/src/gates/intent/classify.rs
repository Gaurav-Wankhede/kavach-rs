//! Prompt-time classification helpers: injection block, FOCUS pin, and the
//! RAG-narrowing + invocable-filtering of the classifier's `required_skills`.
use kavach_session::SessionState;

/// `Some(block-message)` when the prompt trips the prompt-injection guard.
/// SOURCE: Hermes Agent allowlist + OWASP LLM01 Prompt Injection.
pub(super) fn prompt_injection_block(prompt: &str) -> Option<String> {
    let hit = kavach_patterns::prompt_injection_guard::first_blocking_hit(prompt)?;
    Some(format!(
        "[PROMPT_INJECTION] pattern: {}\ncategory: {:?}\ndescription: {}\nmatched: \"{}\"\n\nThis prompt contains patterns associated with prompt injection attacks \
         -> if this is a false positive, rephrase without system-override language -> retry.",
        hit.pattern_name, hit.category, hit.description, hit.matched_text,
    ))
}

/// §FOCUS — DETERMINISTIC pin/clear of the user's supreme scope. ONLY an
/// explicit `FOCUS:` line marker sets it (never inferred from prose).
/// `FOCUS:CLEAR`/`FOCUS:DONE` clears the pin; `FOCUS: <text>` pins it.
pub(super) fn apply_focus_marker(session: &mut SessionState, prompt: &str) {
    let Some(line) = prompt
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("FOCUS:"))
    else {
        return;
    };
    let arg = line.get("FOCUS:".len()..).map_or("", str::trim);
    if arg.eq_ignore_ascii_case("CLEAR") || arg.eq_ignore_ascii_case("DONE") {
        session.user_focus.clear();
    } else if !arg.is_empty() {
        session.user_focus.clone_from(&arg.to_owned());
    }
    session.save_or_log();
}

/// Drop `user-invocable: false` skills — requiring one creates an unresolvable
/// gate deadlock. Missing/unreadable SKILL.md defaults to invocable.
pub(super) fn filter_invocable_skills(skills_raw: Vec<String>) -> Vec<String> {
    let skills_dir = kavach_config::paths::skills_dir();
    skills_raw
        .into_iter()
        .filter(|name| {
            let skill_md = skills_dir.join(name).join("SKILL.md");
            std::fs::read_to_string(&skill_md)
                .map_or(true, |text| !text.contains("user-invocable: false"))
        })
        .collect()
}

/// Collapse the classifier-produced `required_skills` list to the single top
/// RAG-picked entry, if the matcher returns at least one overlap. Returns the
/// original list unchanged when it has <=1 entry, the matcher is cold, or no
/// RAG hit matches any entry in the classifier list.
pub(super) fn collapse_required_via_rag(
    classifier_list: Vec<String>,
    prompt: &str,
    intent_type: &str,
) -> Vec<String> {
    if classifier_list.len() <= 1 {
        return classifier_list;
    }
    let ranking = super::super::rag_router::top_skill_names_all("", prompt, intent_type, 5);
    if ranking.is_empty() {
        return classifier_list;
    }
    for name in &ranking {
        if classifier_list.iter().any(|c| c == name) {
            return vec![name.clone()];
        }
    }
    classifier_list
}

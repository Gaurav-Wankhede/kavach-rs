//! File-pattern skill enforcement for pre-write gate.
//! Checks registry for skills that claim file patterns matching the target.

/// Check if file path triggers any skill enforcement from the registry.
/// Returns Some(reason) if blocked, None if allowed.
pub(crate) fn check_file_pattern_skills(
    file_path: &str,
    session: &kavach_session::SessionState,
) -> Option<String> {
    let cache_path = kavach_config::registry_cache_path();
    let skills_dir = kavach_config::skills_dir();
    let registry = kavach_rule_storage::load_or_rebuild(&cache_path, &skills_dir).ok()?;
    let mut matches = kavach_rule_engine::file_matcher::match_file(file_path, &registry.skills);
    if !matches.has_matches() {
        return None;
    }
    // Narrow the critical list to the single top hit from the vectorless
    // RAG matcher. Demoted criticals become advisories so the caller can
    // still opt in. If no tree is persisted or the matcher finds nothing,
    // the legacy critical list is preserved.
    // Use research_topic as the query text — more signal than an empty string,
    // which causes the RAG scorer to match on file-path tokens alone.
    let rag_text = if session.research_topic.is_empty() {
        file_path
    } else {
        session.research_topic.as_str()
    };
    let rag_ranking =
        super::rag_router::top_skill_names_all(file_path, rag_text, &session.intent_type, 5);
    matches.collapse_by_rag(&rag_ranking);
    let decision =
        kavach_rule_engine::enforce::check_enforcement(&matches, &session.invoked_skills);
    match decision {
        kavach_rule_engine::enforce::EnforcementDecision::Allowed => None,
        blocked @ kavach_rule_engine::enforce::EnforcementDecision::Blocked { .. } => Some(
            kavach_rule_engine::enforce::format_block_reason(&blocked, file_path),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nonmatching_file_returns_none() {
        let session = kavach_session::SessionState::default();
        // A file that doesn't match any skill pattern returns None
        let result = check_file_pattern_skills("README.md", &session);
        assert!(result.is_none());
    }
}

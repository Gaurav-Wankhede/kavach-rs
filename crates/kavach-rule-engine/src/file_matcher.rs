//! Match file paths against skill registry patterns.

use kavach_rule_ast::SkillPriority;
use kavach_rule_storage::RegistryEntry;

/// Result of matching a file path against all registry entries.
#[derive(Debug, Default)]
#[expect(clippy::exhaustive_structs, reason = "result type")]
pub struct FileMatchResult {
    pub critical: Vec<String>,
    pub advisory: Vec<String>,
}

impl FileMatchResult {
    #[must_use]
    pub const fn has_matches(&self) -> bool {
        !self.critical.is_empty() || !self.advisory.is_empty()
    }

    /// Narrow the critical list using an ordered skill ranking from a
    /// vectorless RAG matcher (or any other scorer). When the ranking has
    /// at least one name that is present in `self.critical`, keep only that
    /// top hit as critical and move every other original critical skill
    /// into `advisory`. When the ranking is empty or has no overlap,
    /// `self` is unchanged.
    ///
    /// This collapses the "5 criticals for one .rs file" tax into "1
    /// critical (the RAG-picked one) + the rest as any-one advisory" — the
    /// caller only has to invoke the single most relevant skill, and the
    /// advisory pool lets them optionally invoke one of the others.
    pub fn collapse_by_rag(&mut self, rag_ranking: &[String]) {
        if self.critical.len() <= 1 {
            return;
        }
        let top = match rag_ranking
            .iter()
            .find(|name| self.critical.iter().any(|c| c == *name))
        {
            Some(name) => name.clone(),
            None => return,
        };
        let demoted: Vec<String> = self
            .critical
            .iter()
            .filter(|c| **c != top)
            .cloned()
            .collect();
        self.critical = vec![top];
        for name in demoted {
            if !self.advisory.iter().any(|a| a == &name) {
                self.advisory.push(name);
            }
        }
    }
}

/// Match a file path against all registry entries.
#[must_use]
pub fn match_file(file_path: &str, entries: &[RegistryEntry]) -> FileMatchResult {
    let mut result = FileMatchResult::default();
    for entry in entries {
        if entry_matches(file_path, entry) {
            match entry.priority {
                SkillPriority::Critical => result.critical.push(entry.name.clone()),
                SkillPriority::Advisory => result.advisory.push(entry.name.clone()),
            }
        }
    }
    result
}

fn entry_matches(file_path: &str, entry: &RegistryEntry) -> bool {
    entry
        .file_patterns
        .iter()
        .any(|pattern| glob_match::glob_match(pattern, file_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(name: &str, patterns: &[&str], priority: SkillPriority) -> RegistryEntry {
        RegistryEntry::new(
            name.to_owned(),
            patterns.iter().map(ToString::to_string).collect(),
            priority,
        )
    }

    #[test]
    fn test_match_critical_skill() {
        let entries = vec![make_entry(
            "security",
            &["**/auth/**"],
            SkillPriority::Critical,
        )];
        let result = match_file("src/auth/middleware.rs", &entries);
        assert_eq!(result.critical, vec!["security"]);
        assert!(result.advisory.is_empty());
        assert!(result.has_matches());
    }

    #[test]
    fn test_match_advisory_skill() {
        let entries = vec![make_entry("rust", &["**/*.rs"], SkillPriority::Advisory)];
        let result = match_file("src/main.rs", &entries);
        assert!(result.critical.is_empty());
        assert_eq!(result.advisory, vec!["rust"]);
        assert!(result.has_matches());
    }

    #[test]
    fn test_no_match() {
        let entries = vec![make_entry(
            "security",
            &["**/auth/**"],
            SkillPriority::Critical,
        )];
        let result = match_file("src/main.rs", &entries);
        assert!(result.critical.is_empty());
        assert!(result.advisory.is_empty());
        assert!(!result.has_matches());
    }

    #[test]
    fn should_collapse_criticals_to_top_rag_hit() {
        let mut result = FileMatchResult {
            critical: vec!["rust".into(), "testing".into(), "m06-error-handling".into()],
            advisory: vec!["coding-guidelines".into()],
        };
        result.collapse_by_rag(&["rust".into(), "m07-concurrency".into()]);
        assert_eq!(result.critical, vec!["rust".to_owned()]);
        assert!(result.advisory.contains(&"testing".to_owned()));
        assert!(result.advisory.contains(&"m06-error-handling".to_owned()));
        assert!(result.advisory.contains(&"coding-guidelines".to_owned()));
    }

    #[test]
    fn should_leave_unchanged_when_single_critical() {
        let mut result = FileMatchResult {
            critical: vec!["security".into()],
            advisory: Vec::new(),
        };
        result.collapse_by_rag(&["rust".into()]);
        assert_eq!(result.critical, vec!["security".to_owned()]);
    }

    #[test]
    fn should_leave_unchanged_when_rag_ranking_has_no_overlap() {
        let mut result = FileMatchResult {
            critical: vec!["a".into(), "b".into()],
            advisory: Vec::new(),
        };
        result.collapse_by_rag(&["unrelated".into()]);
        assert_eq!(result.critical.len(), 2);
    }

    #[test]
    fn test_mixed_matches() {
        let entries = vec![
            make_entry("security", &["**/auth/**"], SkillPriority::Critical),
            make_entry("rust", &["**/*.rs"], SkillPriority::Advisory),
            make_entry("domain-web", &["**/handler*"], SkillPriority::Advisory),
        ];
        let result = match_file("src/auth/handler.rs", &entries);
        assert_eq!(result.critical, vec!["security"]);
        assert_eq!(result.advisory.len(), 2);
        assert!(result.advisory.contains(&"rust".to_owned()));
        assert!(result.advisory.contains(&"domain-web".to_owned()));
    }
}

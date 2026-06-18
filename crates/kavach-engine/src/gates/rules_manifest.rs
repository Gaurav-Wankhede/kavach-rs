/// Rules manifest: tracks loaded skill/rule files for context injection.
/// Returns a list of active rule file paths from the session.
/// Reads from ~/.claude-rules (sibling of .claude/) so the directory escapes
/// Claude Code's CLAUDE.md ancestry walk and does not auto-inject ~13K tokens
/// per turn. Falls back to legacy ~/.claude/rules for backward compatibility.
pub(crate) fn active_rules() -> Vec<String> {
    let Ok(home) = std::env::var("HOME") else {
        return vec![];
    };
    let primary = format!("{home}/.claude-rules");
    let legacy = format!("{home}/.claude/rules");
    let rules_dir = if std::path::Path::new(&primary).is_dir() {
        primary
    } else {
        legacy
    };
    // No optimization needed: rules directory is typically <10 files.
    let mut rules = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&rules_dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let Some(name) = file_name.to_str() else {
                continue;
            };
            if std::path::Path::new(name)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("md"))
            {
                rules.push(format!("{rules_dir}/{name}"));
            }
        }
    }
    rules.sort();
    rules
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_active_rules_returns_vec() {
        let rules = active_rules();
        assert!(rules.len() < 1000);
    }
}

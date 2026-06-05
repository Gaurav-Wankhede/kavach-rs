//! Platform-aware path helpers for skills and registry cache.

use std::io::Write;
use std::path::PathBuf;

/// Path to the skills directory (~/.claude/skills/).
#[must_use]
pub fn skills_dir() -> PathBuf {
    dirs::home_dir().map_or_else(
        || {
            drop(writeln!(
                std::io::stderr(),
                "[kavach] WARNING: home_dir() returned None — skill lookups will fail"
            ));
            PathBuf::from("/nonexistent/.claude/skills")
        },
        |home| home.join(".claude").join("skills"),
    )
}

/// Path to the registry cache file (~/.cache/kavach/skill-registry.json).
#[must_use]
pub fn registry_cache_path() -> PathBuf {
    let cache = dirs::cache_dir().or_else(|| dirs::home_dir().map(|h| h.join(".cache")));
    cache.map_or_else(
        || {
            drop(writeln!(
                std::io::stderr(),
                "[kavach] WARNING: no cache or home dir — registry cache disabled"
            ));
            PathBuf::from("/nonexistent/.cache/kavach/skill-registry.json")
        },
        |dir| dir.join("kavach").join("skill-registry.json"),
    )
}

/// Path to the Tailwind Plus component directory (~/.claude/tailwind-plus/).
#[must_use]
pub fn tailwind_plus_dir() -> PathBuf {
    dirs::home_dir().map_or_else(
        || {
            drop(writeln!(
                std::io::stderr(),
                "[kavach] WARNING: home_dir() returned None — tailwind-plus lookups will fail"
            ));
            PathBuf::from("/nonexistent/.claude/tailwind-plus")
        },
        |home| home.join(".claude").join("tailwind-plus"),
    )
}

/// Path to the Tailwind Plus component index (~/.claude/tailwind-plus/index.json).
#[must_use]
pub fn tailwind_plus_index() -> PathBuf {
    tailwind_plus_dir().join("index.json")
}

/// Path to the superpowers specs directory (project-local docs/superpowers/specs/).
#[must_use]
pub fn superpowers_specs_dir() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("docs")
        .join("superpowers")
        .join("specs")
}

/// Path to the GNAP implementation spec.
#[must_use]
pub fn gnap_spec_path() -> PathBuf {
    superpowers_specs_dir().join("2026-04-25-gnap-rfc9635-rfc9767-implementation.md")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skills_dir_ends_with_skills() {
        let path = skills_dir();
        assert!(path.ends_with("skills"));
    }

    #[test]
    fn test_registry_cache_path_ends_with_json() {
        let path = registry_cache_path();
        let name = path.file_name().and_then(|n| n.to_str());
        assert_eq!(name, Some("skill-registry.json"));
    }

    #[test]
    fn test_tailwind_plus_dir_ends_with_tailwind_plus() {
        assert!(tailwind_plus_dir().ends_with("tailwind-plus"));
    }

    #[test]
    fn test_tailwind_plus_index_ends_with_json() {
        let path = tailwind_plus_index();
        let name = path.file_name().and_then(|n| n.to_str());
        assert_eq!(name, Some("index.json"));
    }

    #[test]
    fn test_tailwind_plus_index_inside_tailwind_plus_dir() {
        let dir = tailwind_plus_dir();
        let idx = tailwind_plus_index();
        assert_eq!(idx.parent(), Some(dir.as_path()));
    }
}

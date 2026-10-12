//! YAML frontmatter parsing from TOON skill files

use serde::Deserialize;

fn default_priority() -> String {
    "advisory".into()
}

#[derive(Debug, Clone, Deserialize, Default)]
#[expect(clippy::exhaustive_structs, reason = "constructed cross-crate")]
pub struct FrontmatterMetadata {
    #[serde(default)]
    pub triggers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[expect(clippy::exhaustive_structs, reason = "constructed cross-crate")]
pub struct Frontmatter {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub compatibility: String,
    #[serde(default)]
    pub file_patterns: Vec<String>,
    /// Alias for `file_patterns` used in some SKILL.md files.
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(default)]
    pub metadata: FrontmatterMetadata,
}

impl Frontmatter {
    /// Returns `file_patterns`, falling back to paths if `file_patterns` is empty.
    #[must_use]
    pub fn effective_patterns(&self) -> &[String] {
        if self.file_patterns.is_empty() {
            &self.paths
        } else {
            &self.file_patterns
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[expect(clippy::exhaustive_enums, reason = "matched cross-crate")]
pub enum ParseError {
    #[error("Invalid frontmatter format")]
    InvalidFormat,
    #[error("YAML parsing error: {0}")]
    YamlError(String),
}

/// Parses YAML frontmatter from the beginning of a content string.
///
/// # Errors
/// Returns `ParseError::InvalidFormat` if frontmatter is malformed or delimiters missing.
/// Returns `ParseError::YamlError` if YAML parsing fails.
pub fn parse_frontmatter(content: &str) -> Result<Frontmatter, ParseError> {
    let lines: Vec<&str> = content.lines().collect();

    if !lines.first().is_some_and(|l| l.starts_with("---")) {
        return Err(ParseError::InvalidFormat);
    }

    let yaml_content: String = lines
        .iter()
        .skip(1)
        .take_while(|l| !l.starts_with("---"))
        .copied()
        .collect::<Vec<&str>>()
        .join("\n");

    if yaml_content.trim().is_empty() {
        return Err(ParseError::InvalidFormat);
    }

    serde_yaml::from_str(&yaml_content).map_err(|e| ParseError::YamlError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter_with_patterns() {
        let content = "---\nname: rust\ndescription: Rust skill\nfile_patterns:\n  - \"*.rs\"\n  - \"Cargo.toml\"\npriority: critical\n---\nbody";
        let fm = parse_frontmatter(content).expect("parse frontmatter");
        assert_eq!(fm.name, "rust");
        assert_eq!(fm.file_patterns, vec!["*.rs", "Cargo.toml"]);
        assert_eq!(fm.priority, "critical");
    }

    #[test]
    fn test_parse_frontmatter_without_patterns() {
        let content = "---\nname: general\ndescription: General skill\n---\nbody";
        let fm = parse_frontmatter(content).expect("parse frontmatter");
        assert_eq!(fm.name, "general");
        assert!(fm.file_patterns.is_empty());
        assert_eq!(fm.priority, "advisory");
    }
}

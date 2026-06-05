//! Skill enforcement priority levels.

use serde::{Deserialize, Serialize};

/// Priority level for a skill definition.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[expect(
    clippy::exhaustive_enums,
    reason = "cross-crate literal/match DTO; non_exhaustive => E0639"
)]
pub enum SkillPriority {
    Critical,
    #[default]
    Advisory,
}

impl SkillPriority {
    /// Parse from a string, case-insensitive. Unknown values default to Advisory.
    #[must_use]
    pub fn parse_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "critical" => Self::Critical,
            _ => Self::Advisory,
        }
    }

    /// Returns true if this priority is Critical.
    #[must_use]
    pub const fn is_critical(&self) -> bool {
        matches!(self, Self::Critical)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_priority_is_advisory() {
        assert_eq!(SkillPriority::default(), SkillPriority::Advisory);
        assert!(!SkillPriority::default().is_critical());
    }

    #[test]
    fn test_priority_from_str() {
        assert_eq!(
            SkillPriority::parse_str("critical"),
            SkillPriority::Critical
        );
        assert_eq!(
            SkillPriority::parse_str("CRITICAL"),
            SkillPriority::Critical
        );
        assert_eq!(
            SkillPriority::parse_str("Critical"),
            SkillPriority::Critical
        );
        assert_eq!(
            SkillPriority::parse_str("advisory"),
            SkillPriority::Advisory
        );
        assert_eq!(SkillPriority::parse_str("unknown"), SkillPriority::Advisory);
        assert_eq!(SkillPriority::parse_str(""), SkillPriority::Advisory);
        assert!(SkillPriority::parse_str("critical").is_critical());
        assert!(!SkillPriority::parse_str("advisory").is_critical());
    }
}

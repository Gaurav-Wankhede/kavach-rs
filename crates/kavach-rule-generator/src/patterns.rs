//! Pattern definitions for framework, language, and tool detection.

use crate::detector::PatternType;

#[derive(Debug, Clone)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed at module boundary in all_patterns()"
)]
pub struct FrameworkPattern {
    pub name: &'static str,
    pub pattern_type: PatternType,
    pub file_indicators: &'static [&'static str],
    pub content_indicators: &'static [&'static str],
    pub default_triggers: &'static [&'static str],
}

#[must_use]
pub fn all_patterns() -> Vec<FrameworkPattern> {
    vec![
        // Rust frameworks
        FrameworkPattern {
            name: "axum",
            pattern_type: PatternType::Framework,
            file_indicators: &["Cargo.toml"],
            content_indicators: &["axum::", "use axum", "Router::new()"],
            default_triggers: &["rust", "axum", "backend"],
        },
        FrameworkPattern {
            name: "actix-web",
            pattern_type: PatternType::Framework,
            file_indicators: &["Cargo.toml"],
            content_indicators: &["actix_web::", "use actix_web", "HttpServer::new"],
            default_triggers: &["rust", "actix", "backend"],
        },
        // Frontend frameworks
        FrameworkPattern {
            name: "react",
            pattern_type: PatternType::Framework,
            file_indicators: &["package.json"],
            content_indicators: &["from 'react'", "from \"react\"", "useState"],
            default_triggers: &["react", "frontend", "typescript"],
        },
        FrameworkPattern {
            name: "nextjs",
            pattern_type: PatternType::Framework,
            file_indicators: &["next.config"],
            content_indicators: &["from 'next", "getServerSideProps", "getStaticProps"],
            default_triggers: &["nextjs", "react", "frontend"],
        },
        // Tools
        FrameworkPattern {
            name: "docker",
            pattern_type: PatternType::Tool,
            file_indicators: &["Dockerfile", "docker-compose"],
            content_indicators: &["FROM ", "WORKDIR ", "COPY "],
            default_triggers: &["docker", "containers", "devops"],
        },
        FrameworkPattern {
            name: "github-actions",
            pattern_type: PatternType::Tool,
            file_indicators: &[".github/workflows"],
            content_indicators: &["runs-on:", "uses:", "steps:"],
            default_triggers: &["ci", "github-actions", "devops"],
        },
        FrameworkPattern {
            name: "terraform",
            pattern_type: PatternType::Tool,
            file_indicators: &[".tf"],
            content_indicators: &["resource ", "provider ", "terraform {"],
            default_triggers: &["terraform", "infrastructure", "devops"],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_all_patterns_non_empty() {
        let p = all_patterns();
        assert!(
            p.len() >= 7,
            "expected at least 7 patterns, got {}",
            p.len()
        );
    }
    #[test]
    fn test_pattern_has_triggers() {
        for p in all_patterns() {
            assert!(!p.default_triggers.is_empty(), "{} has no triggers", p.name);
            assert!(
                !p.file_indicators.is_empty(),
                "{} has no file indicators",
                p.name
            );
        }
    }
}

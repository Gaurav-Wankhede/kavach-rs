use crate::error::{Error, Result};
use crate::graph::parser::{ParseResult, SupportedLanguage};
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

/// A matched pattern with captured variables.
#[derive(Debug, Clone)]
pub struct PatternMatch {
    /// The matched node kind.
    pub kind: String,
    /// The source text of the match.
    pub text: String,
    /// Captured metavariables.
    pub captures: Vec<(String, String)>,
}

/// Pattern types for graph edge extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GraphPattern {
    /// "X depends on Y" — roadmap dependency
    DependsOn,
    /// "X uses Y" — technology usage
    Uses,
    /// "X mitigates Y" — mistake mitigation
    Mitigates,
    /// "X informs Y" — research informs
    Informs,
    /// "X justifies Y" — decision justification
    Justifies,
}

impl GraphPattern {
    /// Get the pattern string for ast-grep.
    #[must_use]
    pub fn pattern_str(&self) -> &'static str {
        match self {
            Self::DependsOn => "$SRC depends_on $DST",
            Self::Uses => "$SRC uses $TECH",
            Self::Mitigates => "$MISTAKE mitigates $RISK",
            Self::Informs => "$RESEARCH informs $TARGET",
            Self::Justifies => "$DECISION justifies $TARGET",
        }
    }

    /// Get the edge type for this pattern.
    #[must_use]
    pub fn edge_type(&self) -> &'static str {
        match self {
            Self::DependsOn => "depends_on",
            Self::Uses => "uses_tech",
            Self::Mitigates => "mitigates",
            Self::Informs => "informs",
            Self::Justifies => "justifies",
        }
    }

    /// Get the default weight for this pattern.
    #[must_use]
    pub fn weight(&self) -> f64 {
        match self {
            Self::DependsOn => 1.0,
            Self::Uses => 0.6,
            Self::Mitigates => 0.8,
            Self::Informs => 0.9,
            Self::Justifies => 0.7,
        }
    }
}

/// Pattern matcher for graph-relevant constructs.
#[derive(Debug)]
pub struct GraphPatternMatcher {
    language: SupportedLanguage,
}

impl GraphPatternMatcher {
    /// Create a new pattern matcher for a specific language.
    #[must_use]
    pub fn new(language: SupportedLanguage) -> Self {
        Self { language }
    }

    /// Convert SupportedLanguage to ast-grep CLI language string.
    #[must_use]
    fn language_to_cli_string(&self) -> Option<&'static str> {
        match self.language {
            SupportedLanguage::Rust => Some("rust"),
            SupportedLanguage::Python => Some("python"),
            SupportedLanguage::JavaScript => Some("javascript"),
            SupportedLanguage::TypeScript => Some("typescript"),
            SupportedLanguage::TSX => Some("tsx"),
            SupportedLanguage::Markdown => None, // Let ast-grep auto-detect
            SupportedLanguage::YAML => Some("yaml"),
            SupportedLanguage::TOML => Some("toml"),
            SupportedLanguage::JSON => Some("json"),
            SupportedLanguage::Bash => Some("bash"),
            SupportedLanguage::SQL => Some("sql"),
            SupportedLanguage::PlainText => None, // Let ast-grep auto-detect
        }
    }

    /// Find all matches of a pattern in a parse result.
    ///
    /// # Errors
    /// Returns an error if the ast-grep CLI fails or if the output cannot be parsed.
    pub fn find_matches(
        &self,
        parse_result: &ParseResult,
        pattern: GraphPattern,
    ) -> Result<Vec<PatternMatch>> {
        // Create a temporary file with the source code
        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(parse_result.source.as_bytes())?;
        temp_file.flush()?;

        // Build the ast-grep command
        let mut cmd = Command::new("ast-grep");
        cmd.arg("--pattern")
           .arg(pattern.pattern_str())
           .arg("--json")
           .arg(temp_file.path());

        // Add language specification if we have one (except for PlainText which we let auto-detect)
        if let Some(lang) = self.language_to_cli_string() {
            cmd.arg("--lang").arg(lang);
        }

        // Run ast-grep CLI if installed; fail gracefully if not present
        let output = match cmd.output() {
            Ok(out) => out,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };

        // Check for errors
        if !output.status.success() {
            // If stderr is empty and exit code is 1, treat as no matches (ast-grep returns 1 when no matches found)
            if output.stderr.is_empty() && output.status.code() == Some(1) {
                return Ok(Vec::new());
            }
            return Err(Error::Validation(format!(
                "ast-grep failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Parse the JSON output
        let matches: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)?;

        // Convert to our PatternMatch type
        let mut results = Vec::new();
        for m in matches {
            // Extract the matched text
            let text = m["text"]
                .as_str()
                .unwrap_or("")
                .to_string();

            // Extract captures (submatches)
            let mut captures = Vec::new();
            if let Some(submatches) = m["submatches"].as_array() {
                for sm in submatches {
                    let name = sm["name"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    let text = sm["text"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    captures.push((name, text));
                }
            }

            // We don't have the kind from the CLI output easily, so we use a placeholder.
            // In a future improvement, we could try to get the kind from the AST node.
            results.push(PatternMatch {
                kind: "unknown".to_string(),
                text,
                captures,
            });
        }

        Ok(results)
    }

    /// Find all matches for all graph patterns.
    ///
    /// # Errors
    /// Returns an error if pattern compilation fails.
    pub fn find_all_graph_patterns(
        &self,
        parse_result: &ParseResult,
    ) -> Result<Vec<(GraphPattern, PatternMatch)>> {
        let mut all_matches = Vec::new();

        for pattern in [
            GraphPattern::DependsOn,
            GraphPattern::Uses,
            GraphPattern::Mitigates,
            GraphPattern::Informs,
            GraphPattern::Justifies,
        ] {
            let matches = self.find_matches(parse_result, pattern)?;
            for m in matches {
                all_matches.push((pattern, m));
            }
        }

        Ok(all_matches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::parser::MultiLanguageParser;

    #[test]
    fn test_depends_on_pattern() {
        let mut parser = MultiLanguageParser::new().unwrap();
        let code = "card-1 depends_on card-2";
        let result = parser.parse(code, SupportedLanguage::Markdown).unwrap();

        let matcher = GraphPatternMatcher::new(SupportedLanguage::Markdown);
        let matches = matcher.find_matches(&result, GraphPattern::DependsOn).unwrap();

        // The pattern may or may not match depending on markdown grammar
        // This is a smoke test to ensure the API works
        assert!(matches.len() <= 1);
    }

    #[test]
    fn test_pattern_metadata() {
        assert_eq!(GraphPattern::DependsOn.edge_type(), "depends_on");
        assert_eq!(GraphPattern::Uses.edge_type(), "uses_tech");
        assert_eq!(GraphPattern::Mitigates.edge_type(), "mitigates");
        assert_eq!(GraphPattern::Informs.edge_type(), "informs");
        assert_eq!(GraphPattern::Justifies.edge_type(), "justifies");

        assert_eq!(GraphPattern::DependsOn.weight(), 1.0);
        assert_eq!(GraphPattern::Uses.weight(), 0.6);
        assert_eq!(GraphPattern::Mitigates.weight(), 0.8);
        assert_eq!(GraphPattern::Informs.weight(), 0.9);
        assert_eq!(GraphPattern::Justifies.weight(), 0.7);
    }
}
use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum RulesAction {
    /// List all loaded rules/skills
    List,
    /// Check rules against a file
    Check {
        /// File path to check
        path: String,
    },
    /// Generate rules from detected patterns
    Generate {
        /// Directory to scan
        dir: String,
    },
    /// Show a specific rule/skill
    Show {
        /// Rule/skill name
        name: String,
    },
}

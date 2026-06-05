use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum SecurityAction {
    /// Initialize security context (threat model, auth flows, known false positives)
    Init {
        /// Project root path
        #[arg(long, default_value = ".")]
        path: String,
    },
    /// Fast regex pre-filter scan for security-sensitive patterns
    Scan {
        /// Project root path
        #[arg(long, default_value = ".")]
        path: String,
        /// Output file for filtered results (default: .kavach/security-scan.json)
        #[arg(long)]
        output: Option<String>,
    },
    /// LLM deep analysis on filtered files (batched, resumable)
    Process {
        /// Input file from scan phase (default: .kavach/security-scan.json)
        #[arg(long)]
        input: Option<String>,
        /// Batch size for parallel processing
        #[arg(long, default_value_t = 5)]
        batch_size: usize,
        /// Resume from last checkpoint
        #[arg(long)]
        resume: bool,
    },
    /// Generate security report with git metadata and fix recommendations
    Report {
        /// Output format: markdown | json
        #[arg(long, default_value = "markdown")]
        format: String,
        /// Output file (default: stdout)
        #[arg(long)]
        output: Option<String>,
    },
}

use clap::{Subcommand, ValueEnum};

#[derive(Copy, Clone, Debug, ValueEnum)]
pub(crate) enum OversizedFormat {
    Text,
    Json,
}

#[derive(Subcommand)]
pub(crate) enum OversizedAction {
    /// Scan a directory for files whose code-LOC exceeds the threshold.
    /// Uses `tokei` to count code lines (excludes comments + blanks).
    Scan {
        /// Root directory to scan (defaults to current working directory)
        #[arg(long, default_value = ".")]
        dir: String,
        /// Code-LOC threshold. Files at or below this are silent.
        /// 3-tier hierarchy: <=100 OK, >100 P1 advise, >200 P0 block, >500 urgent.
        #[arg(long, default_value_t = 100)]
        threshold: u32,
        /// Output format
        #[arg(long, value_enum, default_value_t = OversizedFormat::Text)]
        format: OversizedFormat,
    },
}

// ARCH: see kavach db get --category decision --key arch.decision.fourteen_prefix_const_table

use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum SpecAction {
    /// Auto-draft a missing spec artifact
    AutoDraft {
        /// Prefix to draft (e.g. spec.prd, arch.api)
        prefix: String,
        /// Project slug
        #[arg(long)]
        project: String,
    },
}

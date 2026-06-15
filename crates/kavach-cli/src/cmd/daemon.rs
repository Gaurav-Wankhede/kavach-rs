// kavach daemon — code-owned launchd LaunchAgent management for the RPC daemon.
// `install` generates the plist with ORT_DYLIB_PATH baked in, replacing the
// hand-edited plist that made the embedder runtime non-portable.
// SOURCE: decision.embedder-ort-dylib-in-process-resolver.
mod install;

use clap::{Args, Subcommand};

#[derive(Debug, Args)]
#[command(
    about = "Manage the launchd RPC-daemon LaunchAgent (code-owned plist generation)",
    after_help = "EXAMPLES:\n  \
kavach daemon install --dry-run    # render the plist to stdout\n  \
kavach daemon install              # write ~/Library/LaunchAgents/ai.shared.kavach-rpc.plist\n\n\
WHEN: fresh-machine setup or after the ORT runtime is staged — bakes ORT_DYLIB_PATH \
into the plist so the embedder works with no manual edits."
)]
pub(crate) struct DaemonArgs {
    #[command(subcommand)]
    pub action: DaemonAction,
}

#[derive(Debug, Subcommand)]
pub(crate) enum DaemonAction {
    /// Generate the `LaunchAgent` plist with a resolved `ORT_DYLIB_PATH`.
    Install {
        /// Render the plist to stdout instead of writing it (no filesystem change).
        #[arg(long)]
        dry_run: bool,
    },
}

pub(crate) fn run(args: &DaemonArgs) -> i32 {
    match args.action {
        DaemonAction::Install { dry_run } => install::run(dry_run),
    }
}

use clap::Subcommand;

/// Actions for `kavach tailwind-plus <action>`.
#[derive(Subcommand, Copy, Clone)]
pub(crate) enum TailwindPlusAction {
    /// Walk ~/.claude/tailwind-plus/, extract keywords, write index.json.
    Index,
}

// CLI binary: stdout/stderr ARE the product (no tracing sink); process::exit is
// the conventional bin exit-code path; pub(crate) across the private module tree
// aids cross-module reference and is a known nursery false-positive for binaries.
// SOURCE: https://rust-lang.github.io/rust-clippy/master/index.html#redundant_pub_crate (nursery).
#![expect(
    clippy::print_stdout,
    reason = "CLI binary: stdout is the user-facing output channel"
)]
#![expect(
    clippy::print_stderr,
    reason = "CLI binary: stderr is the diagnostic output channel"
)]
#![expect(
    clippy::redundant_pub_crate,
    reason = "binary crate: pub(crate) marks cross-module-internal items; no external surface to leak"
)]

mod cli;
// `pub(crate)` so `cli::db` can reference `cmd::db::write::CATEGORY_HELP` —
// the single-source-of-truth for the --category clap help
// (rca.kavach-db-write-category-enum-inconsistent). Crate-internal only;
// no external API surface change.
pub(crate) mod cmd;

use clap::Parser;

// SOURCE: https://docs.rs/color-eyre (v0.6, 2026)
// Install panic + error report hooks once at process start.
// Provides ANSI-rendered backtraces with file:line context on panic.
fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let args = cli::Cli::parse();
    let code = cmd::dispatch(args.command);
    std::process::exit(code);
}

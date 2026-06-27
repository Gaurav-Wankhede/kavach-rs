//! Rust CLI Toolbelt — kavach orchestrates these as working hands.
//!
//! Maps legacy commands to faster Rust alternatives:
//! - grep → rg (ripgrep) ~10x faster
//! - find → fd ~5x faster
//! - cat → bat (syntax highlighting)
//! - sed → sd (simpler syntax)
//! - diff → difft (AST-aware)
//! - jq → jaq (Rust safety)
//! - du → dust (visual)
//! - ps → procs (searchable)
//! - curl → xh (colored JSON)
//! - cloc → tokei ~10x faster
//!
//! SOURCE: <https://github.com/sts10/rust-command-line-utilities>
//!
//! hub: re-exports the `Tool` enum + every command wrapper. Resolution/cache
//! lives in submodules grouped by concern (search, files, net, proc).
mod cache;
mod files;
mod net;
mod proc;
mod search;
mod tool;
#[cfg(test)]
#[path = "toolbelt_test.rs"]
#[cfg(test)]
mod tests;
pub use files::{count_lines, diff, disk_usage, read_file, tree};
pub use net::verify_url_reachable;
pub use proc::{git_diff_stat, git_has_pending_changes};
pub use search::search;
pub use tool::Tool;

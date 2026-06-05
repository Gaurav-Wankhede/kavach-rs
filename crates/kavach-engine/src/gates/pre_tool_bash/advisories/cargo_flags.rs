//! Multi-crate cargo advisory: warn when one `cargo check`/`build` targets
//! multiple `-p` crates (the slowest way to check several crates). Quote-aware +
//! value-flag-aware so it counts only genuine package selectors.
use crate::gates::pre_tool_bash::strip_quoted_regions;

/// Value-taking flags whose *next* token is an argument, not a `-p` selector.
/// Superset of cargo-check/build value-flags plus nextest's `-E`/`-T`.
/// SOURCE: <https://doc.rust-lang.org/cargo/commands/cargo-check.html>
const VALUE_FLAGS: &[&str] = &[
    "-E",
    "--filter-expr",
    "-m",
    "--message-format",
    "--target",
    "--manifest-path",
    "--target-dir",
    "--color",
    "--config",
    "--profile",
    "-Z",
    "-F",
    "--features",
    "-j",
    "--jobs",
    "--exclude",
    "-T",
    "--test-threads",
    "--test",
    "--bench",
    "-C",
];

/// Count genuine `-p`/`--package` flag occurrences in a cargo arg list.
///
/// A naive filter over-counts: a `-p` token that is the *value* of a preceding
/// value-taking flag is not a package selector. Scan left-to-right, skip the
/// argument of known value-flags, honor the glued `-p=foo` form, and stop at the
/// `--` end-of-options marker.
fn count_package_flags(cmd: &str) -> usize {
    let mut count = 0_usize;
    let mut tokens = cmd.split_whitespace();
    while let Some(tok) = tokens.next() {
        if tok == "--" {
            break;
        }
        if tok == "-p" || tok == "--package" {
            count = count.saturating_add(1);
            continue;
        }
        if tok.starts_with("-p=") || tok.starts_with("--package=") {
            count = count.saturating_add(1);
            continue;
        }
        // A value-flag in separated form consumes the following token so a
        // `-p` sitting there is not mistaken for a package selector.
        if VALUE_FLAGS.contains(&tok) {
            let _ = tokens.next();
        }
    }
    count
}

/// Advisory: warn when cargo check/build targets multiple `-p` crates at once.
pub(in crate::gates::pre_tool_bash) fn check_multi_crate(cmd: &str) -> Option<String> {
    // Erase quoted spans first: a `git commit -m "…cargo check -p a -p b…"`
    // body must not register as a cargo invocation, nor its `-p` as flags.
    let scrubbed = strip_quoted_regions(cmd);
    if !(scrubbed.contains("cargo check") || scrubbed.contains("cargo build")) {
        return None;
    }
    let p_count = count_package_flags(&scrubbed);
    if p_count < 2 {
        return None;
    }
    Some(format!(
        "[MULTI_CRATE_CHECK] {p_count} -p flags detected on a single cargo invocation.\n\
         Cargo resolves the full dependency graph for all targets in one process — \
         this is the slowest way to check multiple crates.\n\
         FIX: Run one command per crate so Cargo can parallelise:\n\
             cargo check -p crate-a\n\
             cargo check -p crate-b\n\
         Or use --workspace to let Cargo schedule the full graph optimally."
    ))
}

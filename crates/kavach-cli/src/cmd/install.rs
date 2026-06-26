//! `kavach install --vendor <cc|cursor|codex|gemini|pi|all>` — make a native tool
//! load Kavach via ITS OWN official hook config. Reads the embedded template,
//! pins the absolute binary path, writes safely (backup + idempotent + dry-run).
//! SOURCE: decision.kavach-universal-subscription-substrate · roadmap
//! universal.install-official-config.

mod vendor;
mod write;

use crate::cmd::io_safe::{into_exit_code, print_or_exit};
use vendor::Target;

/// Resolve `--vendor` to the target set: `all` expands to the shipped templates;
/// a single tag maps to one target; an unknown tag is rejected (fail-closed).
fn resolve(tag: &str) -> Result<Vec<Target>, String> {
    if tag.eq_ignore_ascii_case("all") {
        return Ok(Target::all().to_vec());
    }
    Target::from_tag(tag).map(|t| vec![t]).ok_or_else(|| {
        format!("unknown --vendor '{tag}' (expected: cc|cursor|codex|gemini|pi|all)")
    })
}

/// Absolute path to the currently-running kavach binary, for pinning into hooks.
fn self_binary() -> std::io::Result<std::path::PathBuf> {
    std::env::current_exe()
}

/// Install one target. Returns the human report line, or an error string when
/// HOME is unset or the write fails. All five vendors ship a template (a future
/// not-yet-built target would reintroduce an `Option` on `Target::template`).
fn install_one(t: Target, binary: &std::path::Path, dry_run: bool) -> Result<String, String> {
    let tpl = t.template();
    let home = std::env::var_os("HOME").ok_or_else(|| "$HOME is unset".to_owned())?;
    let path = std::path::Path::new(&home).join(t.rel_config_path());
    let body = write::render(tpl, binary);
    let _ = t.is_toml(); // reserved: TOML append-merge lands with Codex parity work
    match write::install(&path, &body, dry_run) {
        Ok(outcome) => Ok(format!("[{}] {} -> {outcome:?}", t.name(), path.display())),
        Err(e) => Err(format!("{}: write {}: {e}", t.name(), path.display())),
    }
}

/// `kavach install` entry. `vendor` is the `--vendor` tag; `dry_run` previews.
pub(crate) fn run(vendor: &str, dry_run: bool) -> i32 {
    let targets = match resolve(vendor) {
        Ok(t) => t,
        Err(msg) => return report_err(&format!("kavach install: {msg}")),
    };
    let binary = match self_binary() {
        Ok(b) => b,
        Err(e) => return report_err(&format!("kavach install: locate self: {e}")),
    };

    let mut failed = false;
    for t in targets {
        match install_one(t, &binary, dry_run) {
            Ok(line) => {
                if let Err(io) = print_or_exit(&line) {
                    return into_exit_code(io);
                }
            }
            Err(msg) => {
                failed = true;
                if report_err(&format!("kavach install: {msg}")) != 1 {
                    // report_err already returned the IO exit code path
                }
            }
        }
    }
    i32::from(failed)
}

/// stderr report -> exit 1 (collapses to the IO exit code if stderr itself fails).
fn report_err(msg: &str) -> i32 {
    crate::cmd::io_safe::ewrite_or_exit(msg).map_or_else(into_exit_code, |()| 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_all_expands_to_shipped() {
        assert_eq!(resolve("all").unwrap().len(), Target::all().len());
    }

    #[test]
    fn resolve_single_known() {
        assert_eq!(resolve("cursor").unwrap(), vec![Target::Cursor]);
        assert_eq!(resolve("CC").unwrap(), vec![Target::ClaudeCode]);
    }

    #[test]
    fn resolve_unknown_is_error() {
        assert!(resolve("bogus").is_err());
    }

    #[test]
    fn pi_now_ships_and_dry_run_succeeds() {
        // Pi's TS extension template now ships, so install_one resolves a template
        // and the dry-run reports a DryRun outcome (no longer the unshipped error).
        let bin = std::path::Path::new("/x/kavach");
        let line = install_one(Target::Pi, bin, true).unwrap();
        assert!(line.contains("[pi]"), "{line}");
        assert!(
            line.contains("index.ts"),
            "Pi installs to the extension path: {line}"
        );
    }
}

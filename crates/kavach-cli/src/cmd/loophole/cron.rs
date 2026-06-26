//! `kavach loophole cron` — the PROACTIVE host trigger of the Meta-Harness
//! Loophole Loop. Generates a code-owned launchd `LaunchAgent` plist that runs
//! `kavach loophole loop --project <p>` on a daily calendar interval, mirroring
//! the daemon-install pattern (no hand-edited plists). This is the third trigger
//! (on-demand CLI + stop-gate hook + this cron) the loop runs under.
//! SOURCE: roadmap meta.unit.loophole-e2e-cron-verify · decision.meta.loophole-loop-extends-goal-yaml.

use std::path::PathBuf;

use crate::cmd::io_safe::{IoExit, into_exit_code, print_or_exit};

/// launchd label for the proactive loophole-loop agent (distinct from the RPC
/// daemon's `ai.shared.kavach-rpc`).
const LABEL: &str = "ai.shared.kavach-loophole";

/// Run `kavach loophole cron`. Renders the daily `LaunchAgent` plist (to stdout
/// with `--dry-run`, or to `~/Library/LaunchAgents/<LABEL>.plist`).
pub(crate) fn run(project: &str, hour: u8, dry_run: bool) -> i32 {
    match run_inner(project, hour, dry_run) {
        Ok(()) => 0,
        Err(io) => into_exit_code(io),
    }
}

fn run_inner(project: &str, hour: u8, dry_run: bool) -> Result<(), IoExit> {
    // boundary: an out-of-range hour would render a plist launchd silently ignores;
    // clamp to 0–23 and name the clamp rather than emit a dead schedule.
    let safe_hour = hour.min(23);
    if safe_hour != hour {
        print_or_exit(&format!(
            "[loophole cron] hour {hour} out of range; clamped to {safe_hour}"
        ))?;
    }
    let Some(binary) = std::env::current_exe().ok().filter(|p| p.is_absolute()) else {
        return print_or_exit("kavach loophole cron: cannot resolve the kavach binary path");
    };
    let plist = render_plist(&binary.to_string_lossy(), project, safe_hour);

    if dry_run {
        return print_or_exit(&plist);
    }
    let Some(target) = plist_target() else {
        return print_or_exit("kavach loophole cron: HOME not set; cannot locate LaunchAgents dir");
    };
    if let Some(parent) = target.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        return print_or_exit(&format!(
            "kavach loophole cron: cannot create {}: {e}",
            parent.display()
        ));
    }
    match std::fs::write(&target, &plist) {
        Ok(()) => print_or_exit(&format!(
            "wrote {} ({} bytes)\nnext: launchctl bootstrap gui/$(id -u) {}",
            target.display(),
            plist.len(),
            target.display()
        )),
        Err(e) => print_or_exit(&format!(
            "kavach loophole cron: write {} failed: {e}",
            target.display()
        )),
    }
}

/// `~/Library/LaunchAgents/<LABEL>.plist` (macOS launchd user-agent location).
fn plist_target() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join("Library/LaunchAgents")
            .join(format!("{LABEL}.plist")),
    )
}

/// Render the daily `LaunchAgent` plist. `StartCalendarInterval` (hour, minute 0)
/// fires `kavach loophole loop --project <p>` once a day. `RunAtLoad=false` so the
/// install itself does not trigger an immediate sweep; the loop is bounded by its
/// own `--max-rounds` brake. The project + run-id are baked in for idempotency.
fn render_plist(binary: &str, project: &str, hour: u8) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
         \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n<dict>\n  \
         <key>Label</key>\n  <string>{LABEL}</string>\n  \
         <key>ProgramArguments</key>\n  <array>\n    \
         <string>{bin}</string>\n    <string>loophole</string>\n    <string>loop</string>\n    \
         <string>--project</string>\n    <string>{proj}</string>\n    \
         <string>--run-id</string>\n    <string>cron</string>\n  </array>\n  \
         <key>StartCalendarInterval</key>\n  <dict>\n    \
         <key>Hour</key>\n    <integer>{hour}</integer>\n    \
         <key>Minute</key>\n    <integer>0</integer>\n  </dict>\n  \
         <key>RunAtLoad</key>\n  <false/>\n</dict>\n</plist>\n",
        bin = xml_escape(binary),
        proj = xml_escape(project),
    )
}

/// Minimal XML escaping for plist string values (paths/slugs can contain `&`, `<`).
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::{render_plist, xml_escape};

    #[test]
    fn plist_carries_label_project_and_schedule() {
        let p = render_plist("/usr/local/bin/kavach", "kavach-rs", 4);
        assert!(p.contains("ai.shared.kavach-loophole"));
        assert!(p.contains("<string>loophole</string>"));
        assert!(p.contains("<string>loop</string>"));
        assert!(p.contains("<string>kavach-rs</string>"));
        assert!(p.contains("<key>Hour</key>\n    <integer>4</integer>"));
        // RunAtLoad must be false — install must not trigger an immediate sweep.
        assert!(p.contains("<key>RunAtLoad</key>\n  <false/>"));
    }

    #[test]
    fn xml_escape_handles_special_chars() {
        assert_eq!(xml_escape("a&b<c>d"), "a&amp;b&lt;c&gt;d");
    }
}

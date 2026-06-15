//! `kavach daemon install` — generate the launchd `LaunchAgent` plist with a
//! code-owned `ORT_DYLIB_PATH`, replacing the hand-edited plist that made the
//! embedder runtime non-portable.
//!
//! The historical setup required a human to (a) stage `libonnxruntime.dylib` and
//! (b) hand-write `ORT_DYLIB_PATH` into the plist. This command makes both a
//! repeatable, code-owned action: it renders the plist (with the resolved dylib
//! path injected as an `EnvironmentVariables` entry) so a fresh checkout reaches
//! a working embedder without manual edits.
//! SOURCE: decision.embedder-ort-dylib-in-process-resolver.

use std::path::PathBuf;

/// The launchd label + plist path the daemon spawn path already bootstraps.
const LABEL: &str = "ai.shared.kavach-rpc";

/// Run `kavach daemon install`. Renders the `LaunchAgent` plist (to `--dry-run`
/// stdout, or to `~/Library/LaunchAgents/<LABEL>.plist` when applied) with the
/// resolved `ORT_DYLIB_PATH` baked in. Returns a process exit code.
pub(crate) fn run(dry_run: bool) -> i32 {
    let Some(binary) = std::env::current_exe().ok().filter(|p| p.is_absolute()) else {
        eprintln!("kavach daemon install: cannot resolve the kavach binary path");
        return 1;
    };
    let dylib = resolved_dylib();
    let plist = render_plist(&binary.to_string_lossy(), dylib.as_deref());

    if dry_run {
        println!("{plist}");
        if dylib.is_none() {
            eprintln!(
                "note: no ONNX runtime staged at {}; the plist omits ORT_DYLIB_PATH \
                 (ort will fall back to its own search). Stage the dylib there to bake it in.",
                kavach_surreal::conventional_dylib_path().display()
            );
        }
        return 0;
    }

    let Some(target) = plist_target() else {
        eprintln!("kavach daemon install: HOME not set; cannot locate LaunchAgents dir");
        return 1;
    };
    if let Some(parent) = target.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        eprintln!("kavach daemon install: cannot create {}: {e}", parent.display());
        return 1;
    }
    match std::fs::write(&target, &plist) {
        Ok(()) => {
            println!("wrote {} ({} bytes)", target.display(), plist.len());
            println!(
                "next: launchctl bootstrap gui/$(id -u) {} (or `kavach` will kickstart it)",
                target.display()
            );
            0
        }
        Err(e) => {
            eprintln!("kavach daemon install: write {} failed: {e}", target.display());
            1
        }
    }
}

/// The dylib path to bake into the plist: the operator override is left to the
/// existing env (resolver returns `None`), else the staged conventional path if
/// present. `None` ⇒ omit the env entry (fail-open; never point at a missing file).
fn resolved_dylib() -> Option<String> {
    kavach_surreal::resolve_dylib_path().map(|p| p.to_string_lossy().into_owned())
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

/// Render the `LaunchAgent` plist. When `dylib` is `Some`, an
/// `EnvironmentVariables` dict carries `ORT_DYLIB_PATH`; when `None`, the dict is
/// omitted so `ort` falls back to its own runtime search. `KeepAlive=true` +
/// `RunAtLoad` mirror the daemon-spawn contract.
fn render_plist(binary: &str, dylib: Option<&str>) -> String {
    let env_block = dylib.map_or_else(String::new, |path| {
        format!(
            "  <key>EnvironmentVariables</key>\n  <dict>\n    \
             <key>{env}</key>\n    <string>{path}</string>\n  </dict>\n",
            env = kavach_surreal::ORT_DYLIB_ENV,
            path = xml_escape(path),
        )
    });
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
         \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n<dict>\n  \
         <key>Label</key>\n  <string>{LABEL}</string>\n  \
         <key>ProgramArguments</key>\n  <array>\n    \
         <string>{bin}</string>\n    <string>rpc</string>\n    \
         <string>--transport</string>\n    <string>http</string>\n  </array>\n\
         {env_block}  \
         <key>RunAtLoad</key>\n  <true/>\n  \
         <key>KeepAlive</key>\n  <true/>\n</dict>\n</plist>\n",
        bin = xml_escape(binary),
    )
}

/// Minimal XML escaping for plist string values (paths can contain `&`, `<`).
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
#[path = "install_test.rs"]
mod install_test;

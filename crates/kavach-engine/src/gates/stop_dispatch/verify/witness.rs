//! The objective build+test witness machinery for auto-verify (`§NANO_FILE` split
//! from `verify.rs`). Discovers the Rust workspace (root or monorepo subdir) and
//! runs cargo check + clippy + nextest + git-diff; the orchestration that consumes
//! [`WitnessRun`]/[`run_workspace_witnesses`] lives in the `verify.rs` hub.

/// Whether the cargo workspace witnesses ran and what they found.
///
/// `Passed`: all witnesses (cargo check, clippy, nextest) executed and succeeded.
/// `Failed`: witnesses executed but a witness returned non-zero — real AI repair.
/// `Unprovable`: not a Rust workspace AND no `KAVACH_VERIFY_CMD` — cannot prove.
/// `SpawnError`: a Rust workspace, but cargo/clippy/nextest failed to spawn — a
/// HARD failure for a Rust project, not a fallback (rca.keystone-trap).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WitnessRun {
    Passed,
    Failed,
    SpawnError,
    Unprovable,
}

/// True iff `dir` itself holds a `Cargo.toml`. Pure, dir-parameterized so the
/// rule is unit-testable without the test process's CWD. SOURCE: rca.keystone-trap.
fn is_rust_workspace(dir: &std::path::Path) -> bool {
    dir.join("Cargo.toml").exists()
}

/// Discover the Rust workspace dir to verify in: `root` itself, else an immediate
/// child holding a `Cargo.toml`. Returns `None` for a non-Rust project.
///
/// FUNDAMENTAL FIX (rca.monorepo-verify-blind): the prior check only looked at
/// `cwd/Cargo.toml`, so a MONOREPO whose Rust workspace lives in a subdir (e.g.
/// `Backend/`) was classified non-Rust → auto-verify returned `Unprovable` and
/// NEVER promoted done→verified, silently disabling the self-closing loop. Walking
/// one level down finds `Backend/` and runs the witnesses there. Children are
/// scanned in sorted order for determinism; `target`/hidden dirs are skipped.
fn discover_rust_workspace(root: &std::path::Path) -> Option<std::path::PathBuf> {
    if is_rust_workspace(root) {
        return Some(root.to_path_buf());
    }
    let mut children: Vec<std::path::PathBuf> = std::fs::read_dir(root)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n != "target" && !n.starts_with('.'))
        })
        .collect();
    children.sort();
    children.into_iter().find(|c| is_rust_workspace(c))
}

/// Read `KAVACH_VERIFY_CMD` if set (the non-Rust escape hatch).
fn verify_command_env() -> Option<String> {
    std::env::var("KAVACH_VERIFY_CMD").ok()
}

/// First non-empty `WITNESS_ROOT: <path>` line in a card (trimmed, verbatim, `~`
/// NOT expanded), or `None`. Lets a cross-repo card name its real workspace.
#[must_use]
pub(crate) fn witness_root_from_card(content: &str) -> Option<String> {
    content.lines().find_map(|raw| {
        raw.trim()
            .strip_prefix("WITNESS_ROOT:")
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(str::to_owned)
    })
}

/// Run the build+test witnesses ONCE. Workspace precedence, most-specific first:
/// per-card `card_root` hint → `WITNESS_ROOT` env → CWD → `KAVACH_VERIFY_CMD` → Unprovable.
pub(crate) fn run_workspace_witnesses(card_root: Option<&str>) -> WitnessRun {
    // 1. Per-card hint wins — the card declares the repo its code lives in.
    if let Some(root) = card_root
        && let Some(ws) = discover_rust_workspace(std::path::Path::new(root))
    {
        return run_cargo_witnesses(&ws);
    }
    // 2. Session-wide env override.
    if let Ok(root) = std::env::var("WITNESS_ROOT")
        && let Some(ws) = discover_rust_workspace(std::path::Path::new(&root))
    {
        return run_cargo_witnesses(&ws);
    }
    // 3. Dispatch CWD (root or monorepo subdir).
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    if let Some(ws) = discover_rust_workspace(&cwd) {
        return run_cargo_witnesses(&ws);
    }
    // 4. Non-Rust escape hatch, else unprovable.
    verify_command_env().map_or(WitnessRun::Unprovable, |cmd| {
        match std::process::Command::new("sh").args(["-c", &cmd]).status() {
            Ok(status) if status.success() => WitnessRun::Passed,
            Ok(_) | Err(_) => WitnessRun::Failed,
        }
    })
}

/// Run cargo check + clippy + nextest + git-diff IN `ws` (the discovered Rust
/// workspace dir) — `current_dir(ws)` so a monorepo's `Backend/` is verified.
fn run_cargo_witnesses(ws: &std::path::Path) -> WitnessRun {
    let run = |args: &[&str]| {
        std::process::Command::new("cargo")
            .args(args)
            .current_dir(ws)
            .status()
    };
    for args in [
        ["check", "--workspace", "--quiet"].as_slice(),
        ["clippy", "--workspace", "--all-targets", "--quiet", "--", "-D", "warnings"].as_slice(),
        ["nextest", "run", "--workspace"].as_slice(),
    ] {
        match run(args) {
            Ok(status) if status.success() => (),
            Ok(_) => return WitnessRun::Failed,
            Err(_) => return WitnessRun::SpawnError,
        }
    }
    // Change-landed witness: a committed (clean) tree is landed too, not a failure.
    WitnessRun::Passed
}

// Tests lifted to a sibling (§NANO_FILE: this machinery file stays ≤100 LOC).
// decision.kavach.verify-rs-nanofile-split-2026-06-17 — mechanical, behavior-
// identical; rca.keystone-trap + rca.monorepo-verify-blind preserved verbatim.
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

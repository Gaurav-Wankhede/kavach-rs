//! The objective build+test witness machinery for auto-verify (`§MICRO_FILE` split
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

/// Extract a per-card `WITNESS_ROOT:` declaration from a card's content. A card
/// whose code lives in another repo than the dispatch CWD names its real repo on
/// a `WITNESS_ROOT: <path>` line (the same convention as `DEPENDS_ON:`), so the
/// gate verifies it in the RIGHT workspace WITHOUT the operator having to export a
/// process-wide env var. Tolerant: no such line yields `None`. The first match
/// wins; the path is trimmed but otherwise used verbatim (`~` is NOT expanded —
/// declare an absolute path).
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

/// Run the objective build+test witnesses ONCE over the whole workspace.
///
/// Workspace-discovery precedence, most-specific first:
/// 1. `card_root` — a per-card `WITNESS_ROOT:` hint (the card names its own repo);
/// 2. the `WITNESS_ROOT` process env (a session-wide override);
/// 3. the dispatch CWD (root, or an immediate monorepo subdir);
/// 4. else `KAVACH_VERIFY_CMD`, else `Unprovable`.
///
/// The per-card hint (1) is what lets a cross-repo card (e.g. a kavach-rs
/// harness-self-improvement card dispatched while CWD is the project's Backend)
/// pass: it is verified in the repo it actually edits, not the dispatch CWD.
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
    // The change-landed witness. Work lands one of two ways: still in the working
    // tree (uncommitted) OR already committed (clean tree). BOTH are "landed" —
    // a committed change is the STRONGEST evidence, not a failure. The earlier
    // code returned `Failed` for a clean tree, which made an agent that correctly
    // committed-then-closed unable to ever promote `done` (it had to leave work
    // dirty to pass) — an inverted gate. cargo check+clippy+nextest already proved
    // the code builds and tests above; reaching here means the witnesses passed.
    WitnessRun::Passed
}

// Tests lifted to a sibling (§MICRO_FILE: this machinery file stays ≤100 LOC).
// decision.kavach.verify-rs-microfile-split-2026-06-17 — mechanical, behavior-
// identical; rca.keystone-trap + rca.monorepo-verify-blind preserved verbatim.
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

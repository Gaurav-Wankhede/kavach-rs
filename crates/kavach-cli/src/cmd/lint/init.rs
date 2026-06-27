// `kavach lint init` core — detect each stack at the workspace root and install
// its strict-rules manifest, fail-soft + never clobber an existing manifest.
// SOURCE: decision.lint.language-profile-template.
use std::path::Path;

use crate::cmd::io_safe;
use crate::cmd::lint::detect::{Stack, detect};
use crate::cmd::lint::profiles::{go, rust, ts};

/// Install (or report) the strict profile for every detected stack at `root`.
/// `dry_run` reports what WOULD be written without touching disk.
pub(crate) fn run(root: &Path, dry_run: bool) -> i32 {
    let stacks = detect(root);
    if stacks.is_empty() {
        return emit("kavach lint: no Rust/TS/Go project detected here — nothing to do.");
    }
    let mut code = 0;
    for stack in stacks {
        if install_one(root, stack, dry_run) != 0 {
            code = 1;
        }
    }
    code
}

/// Install one stack's profile. Rust APPENDS the workspace-lints table to an
/// existing Cargo.toml (idempotent on the marker); TS/Go WRITE a new manifest
/// only when absent. Returns non-zero on a write error (surfaced, never silent).
fn install_one(root: &Path, stack: Stack, dry_run: bool) -> i32 {
    let body = match stack {
        Stack::Rust => rust::RUST_LINTS,
        Stack::Ts => ts::TS_TSCONFIG,
        Stack::Go => go::GO_GOLANGCI,
    };
    let target = root.join(stack.manifest());
    if dry_run {
        return emit(&format!(
            "kavach lint: [dry-run] {} → would install strict {} profile",
            stack.label(),
            target.display()
        ));
    }
    match stack {
        Stack::Rust => append_rust(&target, body),
        Stack::Ts | Stack::Go => write_if_absent(&target, body, stack),
    }
}

/// Append the `[workspace.lints]` table to Cargo.toml unless already present
/// (idempotent — re-running never duplicates). Cargo.toml always exists here
/// (detection keyed on it), so this only ever appends.
fn append_rust(target: &Path, body: &str) -> i32 {
    let existing = std::fs::read_to_string(target).unwrap_or_default();
    if existing.contains("[workspace.lints.rust]") || existing.contains("[lints.rust]") {
        return emit("kavach lint: Rust workspace.lints already present — left unchanged.");
    }
    let merged = format!("{existing}\n{body}");
    match std::fs::write(target, merged) {
        Ok(()) => emit(&format!(
            "kavach lint: appended strict workspace.lints → {}",
            target.display()
        )),
        Err(e) => fail(&format!(
            "kavach lint: write failed {}: {e}",
            target.display()
        )),
    }
}

/// Write a manifest only when absent — never overwrite a project's existing
/// tsconfig/golangci config (the user's tuning wins; we only seed a missing one).
fn write_if_absent(target: &Path, body: &str, stack: Stack) -> i32 {
    if target.exists() {
        return emit(&format!(
            "kavach lint: {} already exists — left unchanged (merge strict opts by hand).",
            stack.manifest()
        ));
    }
    match std::fs::write(target, body) {
        Ok(()) => emit(&format!(
            "kavach lint: wrote strict {} profile → {}",
            stack.label(),
            target.display()
        )),
        Err(e) => fail(&format!(
            "kavach lint: write failed {}: {e}",
            target.display()
        )),
    }
}

fn emit(msg: &str) -> i32 {
    io_safe::print_or_exit(msg).map_or_else(io_safe::into_exit_code, |()| 0)
}

fn fail(msg: &str) -> i32 {
    io_safe::ewrite_or_exit(msg).map_or_else(io_safe::into_exit_code, |()| 1)
}

#[cfg(test)]
#[path = "init_test.rs"]
#[cfg(test)]
#[path = "init_test.rs"]
mod tests;
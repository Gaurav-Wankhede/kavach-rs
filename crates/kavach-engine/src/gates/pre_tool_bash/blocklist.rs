//! Stateless early checks: destructive-CLI guard, config blocklists, write-bypass
//! / git-add advisories, psql/sqlx blocks, unscoped-test + toolbelt advisories.
//! Returns `Some(Decision)` to short-circuit; `None` to fall through to the
//! stateful advisory tail. Ordered so every P0 hard-block precedes advisories.
mod config;

#[cfg(test)]
#[path = "blocklist_test.rs"]
mod tests;

use config::config_blocklists;

use super::advisories::is_git_add_all;
use super::decision::Decision;
use super::test_tracker::check_unscoped_test_run;
use super::write_bypass::{
    check_psql_blocked, check_sqlx_migrate_requires_rca, is_write_bypass, targets_tracked_source,
};

pub(super) fn check(command: &str) -> Option<Decision> {
    destructive_cli(command)
        .or_else(|| config_blocklists(command))
        .or_else(|| bypass_advisories(command))
        .or_else(|| db_blocks(command))
        .or_else(|| test_and_toolbelt(command))
}

/// Shell-syntax-aware destructive guard (P0 deny / P1 confirm; P2 falls through).
fn destructive_cli(command: &str) -> Option<Decision> {
    use kavach_patterns::destructive_cli_guard::DestructiveSeverity::{P0Block, P1Confirm, P2Warn};
    let hit = kavach_patterns::destructive_cli_guard::inspect(command)?;
    let msg = |verb: &str| {
        format!(
            "{verb} [{:?}/{}]: {} — canonical: `{}`",
            hit.category, hit.pattern, hit.fix, hit.canonical
        )
    };
    match hit.severity {
        P0Block => Some(Decision::Deny(msg("BLOCKED"))),
        P1Confirm => Some(Decision::Ask(msg("CONFIRM"))),
        P2Warn => None,
    }
}

/// Write-bypass guard + `git add .` advisory.
///
/// A Bash file-write sidesteps the `pre-write` research / anti-pattern gate
/// (which fires only on Write/Edit). When the target is a TRACKED SOURCE file
/// that is capability-laundering — a hand edit dodging the gate — so it is a
/// P0 DENY. For generated artifacts (configs, `loop.yaml`) the dodge is benign
/// and stays advisory. The `targets_tracked_source` predicate bounds the deny's
/// false-positive surface (it requires both a source tree segment AND a source
/// extension), satisfying the kavach-engine "promote to P0 only with an FP
/// bound" severity policy.
fn bypass_advisories(command: &str) -> Option<Decision> {
    if is_write_bypass(command) {
        if targets_tracked_source(command) {
            return Some(Decision::Deny(
                "[BLOCKED:write-bypass] Writing a tracked SOURCE file via Bash bypasses the \
                 pre-write research / anti-pattern gate. Use the Write or Edit tool so the gate \
                 can mediate the change — do NOT route source edits through python/sed/redirects."
                    .to_owned(),
            ));
        }
        return Some(Decision::Allow(Some(
            "[ADVISORY:write-bypass] File modification via Bash bypasses Write/Edit hooks. \
             Prefer the Write or Edit tool for file changes."
                .to_owned(),
        )));
    }
    if is_git_add_all(command) {
        return Some(Decision::Allow(Some(
            "[ADVISORY:git-add-all] `git add .` stages ALL files including unrelated changes. \
             Prefer `git add <specific-files>` to stage only intended files."
                .to_owned(),
        )));
    }
    None
}

/// psql hard-block + sqlx-migrate RCA advisory.
fn db_blocks(command: &str) -> Option<Decision> {
    if let Some(reason) = check_psql_blocked(command) {
        return Some(Decision::Deny(reason));
    }
    // `sqlx migrate run` against a shared DB requires an [RCA] this turn.
    // Override KAVACH_LOCAL_DB=1 for tests.
    let session = kavach_session::get_or_create_session();
    if let Some(reason) = check_sqlx_migrate_requires_rca(command, session.rca_satisfied()) {
        return Some(Decision::Allow(Some(format!(
            "[ADVISORY:sqlx-migrate-rca] {reason}"
        ))));
    }
    None
}

/// Unscoped-test advisory + the §TOOLBELT legacy-tool advisory.
///
/// The nextest-config scaffold runs FIRST (even for a to-be-denied test
/// command) so the project is left with a tuned `.config/nextest.toml` and the
/// engineer's scoped retry is ready. cwd ≡ `session.work_dir` (not yet loaded).
fn test_and_toolbelt(command: &str) -> Option<Decision> {
    let scaffold_ctx = std::env::current_dir()
        .ok()
        .and_then(|cwd| super::advisories::scaffold_nextest_config(command, &cwd));
    if let Some(reason) = check_unscoped_test_run(command) {
        return Some(Decision::Allow(Some(format!(
            "[ADVISORY:unscoped-test] {reason}"
        ))));
    }
    drop(scaffold_ctx); // side-effect only; the tail re-runs it for context
    if let Some(hit) = kavach_patterns::legacy_tool_guard::inspect(command) {
        return Some(Decision::Deny(format!(
            "[TOOLBELT_BLOCK] `{tool}` is forbidden — use the installed Rust CLI `{repl}` (§TOOLBELT). \
             ALWAYS use the Rust toolbelt for Bash, including when checking existing files.\n\
             Map: grep→rg · find→fd · cat→bat · sed→sd · ls -R→eza · jq→jaq · \
             curl→xh · du→dust · tree→erd · ps→procs · diff→difft · awk→`rg`/`choose` · wc -l→`tokei`.\n\
             Re-run with `{repl}`.",
            tool = hit.tool,
            repl = hit.replacement,
        )));
    }
    None
}

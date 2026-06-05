//! Env-value-leak guard: block commands that expose secret env values to the AI.
//!
//! Allowed: listing variable *names* (`env`, `printenv` no-args, `declare -p`),
//! reading POSIX-standard non-secret system vars (PATH, HOME, USER, SHELL, ...).
//! Blocked: reading a secret value (`printenv DATABASE_URL`, `echo $API_KEY`,
//! `env | grep VAR`, `cat .env`) and loader-injection vars (`LD_*`, `DYLD_*`).
//!
//! FIX `contract_violation`: the gate once blocked `printenv PATH` / `echo $HOME`
//! (POSIX system vars over-blocked) — the contract is "block secret exposure",
//! not "block all env reads". A safe-system-var allowlist now exempts them while
//! loader-injection vars stay blocked.
//! SOURCE: IEEE 1003.1 (POSIX.1-2017) Chapter 8 — Environment Variables;
//! openclaw GHSA-xgf2-vxv2-rrmg (LD_*/DYLD_* are loader-influencing).
//! The nine leak patterns live in `patterns`; this hub chains them, first hit wins.
mod patterns;

/// Block commands that would expose secret env var values to the AI.
pub(crate) fn check_env_value_read(command: &str) -> Option<String> {
    let lc = command.trim().to_lowercase();
    patterns::check_printenv(&lc)
        .or_else(|| patterns::check_echo(&lc))
        .or_else(|| patterns::check_env_grep(&lc))
        .or_else(|| patterns::check_source(&lc, command))
        .or_else(|| patterns::check_set_dump(&lc))
        .or_else(|| patterns::check_python_environ(&lc))
        .or_else(|| patterns::check_proc_environ(&lc))
        .or_else(|| patterns::check_dotenv_read(&lc))
        .or_else(|| patterns::check_dotenv_grep(&lc))
}

// Cross-module re-exports (canonical defs live in the sibling env_guard_* files).
#[cfg(test)]
use super::env_guard_safelist::is_safe_system_var;
#[cfg(test)]
use super::env_guard_shell_parse::skip_shell_redirects;
#[cfg(test)]
use super::env_guard_source_extract::extract_post_source_command;
use super::env_guard_source_extract::has_source_builtin;
pub(crate) use super::env_guard_sourcing::check_env_sourcing;

/// Cross-module access for `env_guard_sourcing` — re-exposes `has_source_builtin`
/// at `pub(crate)` visibility without changing the canonical fn's visibility.
pub(crate) fn has_source_builtin_for_split(lc: &str) -> bool {
    has_source_builtin(lc)
}

#[cfg(test)]
#[path = "env_guard_tests.rs"]
mod tests;

// POSIX-standard non-secret system variables safe to read via echo/printenv.
// See decision.engine.env-guard-safelist-arch.
/// POSIX-standard non-secret system variables — safe to read via `echo` or `printenv`.
///
/// Excludes loader-injection vars (LD_*, DYLD_*, `NODE_OPTIONS`, etc.) per
/// openclaw GHSA-xgf2-vxv2-rrmg threat model.
/// Source: IEEE 1003.1 Chapter 8 + environ(7) Linux manual.
pub(crate) const SAFE_SYSTEM_VARS: &[&str] = &[
    // Identity / location
    "home",
    "user",
    "logname",
    "shell",
    "pwd",
    "oldpwd",
    "tmpdir",
    // Path (read-only is safe; SETTING is what enables hijack — and we don't allow set)
    "path",
    // Locale / i18n
    "lang",
    "lc_all",
    "lc_collate",
    "lc_ctype",
    "lc_messages",
    "lc_monetary",
    "lc_numeric",
    "lc_time",
    "nlspath",
    "tz",
    // Terminal
    "term",
    "columns",
    "lines",
    "colorterm",
    "no_color",
    "force_color",
    // User preference
    "editor",
    "visual",
    "pager",
    // Shell prompts (not secret)
    "ps1",
    "ps2",
    "ps4",
    "ifs",
    // Hostname (commonly-used non-POSIX but standard)
    "hostname",
];
/// Return true when `var_name` is a POSIX-standard non-secret system variable.
///
/// Lookup is O(n) over a small const slice (n=~27, no allocation). `HashSet` would
/// require runtime construction per call.
pub(crate) fn is_safe_system_var(var_name: &str) -> bool {
    let lc = var_name.trim().to_lowercase();
    if lc.is_empty() || lc.len() > 32 {
        return false;
    }
    // LC_* family — match prefix since LC_<anything> follows POSIX naming.
    if lc.starts_with("lc_") {
        return true;
    }
    SAFE_SYSTEM_VARS.contains(&lc.as_str())
}
mod echo;
#[cfg(test)]
#[path = "env_guard_safelist_test.rs"]
#[path = "env_guard_safelist_test.rs"]
mod tests;
pub(crate) use echo::echo_only_references_safe_vars;

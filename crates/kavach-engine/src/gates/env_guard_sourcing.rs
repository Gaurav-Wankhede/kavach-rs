// ARCH: EnvSourcingAdvisory
// PROBLEM_CLASS: env_var_misuse_advisory
// REJECTED: [{"name":"hard-block","reason":"too aggressive — false positive on legitimate env-only execution"},{"name":"silent-allow","reason":"misses real misuse"}]
// TIME: O(n) per command — n = command length | SPACE: O(n) lowercase copy
// YEAR: 2026 | SEARCHED: 2026-05
// TRADEOFF: Substring match for $VAR is approximate (misses indirect refs).
//           Acceptable for advisory output — false negative downgrades to no-msg.
// BENCHMARK: env_guard.rs original tests; preserved across split.
// PATTERN: advisory_message | SCOPE: pre_tool_bash | CAP: AP
// FAILURE_MODE: false negative (advisory not emitted when it should be) → user sees
//               nothing, not unsafe; false positive → noise but harmless.
//
// Extracted from env_guard.rs (split-env-guard-microservices roadmap, May 2026).

/// Check if a bash command references env vars that may need sourcing.
///
/// Returns advisory context if env vars are used without a source/dotenv call.
/// Uses `env_guard`'s `has_source_builtin` so `sqlx --source migrations_local`
/// (argv flag) is never mistaken for the shell `source` builtin.
pub(crate) fn check_env_sourcing(command: &str) -> Option<String> {
    let uses_env_var = command.contains("$DATABASE_URL")
        || command.contains("$SECRET")
        || command.contains("$API_KEY")
        || command.contains("$JWT_SECRET")
        || command.contains("$REDIS_URL");
    let lc = command.to_lowercase();
    let sources_env = super::env_guard::has_source_builtin_for_split(&lc)
        || lc.contains("dotenv")
        || lc.contains("direnv");
    (uses_env_var && !sources_env).then(|| {
        "[ENV_GUARD]\nstatus: advisory\nreason: Command uses env var — ensure .env is sourced\n"
            .into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_env_vars_returns_none() {
        assert!(check_env_sourcing("ls -la").is_none());
    }

    #[test]
    fn env_var_without_source_returns_advisory() {
        assert!(check_env_sourcing("psql $DATABASE_URL").is_some());
    }

    #[test]
    fn env_var_with_source_returns_none() {
        assert!(check_env_sourcing("source .env && psql $DATABASE_URL").is_none());
    }
}

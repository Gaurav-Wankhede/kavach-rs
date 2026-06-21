// Advisory: detect env vars used without .env sourcing.
// See decision.engine.env-guard-sourcing-arch.

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

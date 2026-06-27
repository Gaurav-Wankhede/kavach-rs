// Filename detection + downstream binary allowlist for .env file sourcing.
// See decision.engine.env-guard-dotenv-arch.
//
// `downstream` holds the post-source binary allowlist; `filename` detects the
// `.env*` filename for error messages.
mod downstream;
mod filename;
mod target;
#[cfg(test)]
#[path = "env_guard_dotenv_test.rs"]
mod tests;
pub(crate) use downstream::is_safe_downstream;
pub(crate) use filename::detect_env_filename;
pub(crate) use target::targets_dotenv_file;

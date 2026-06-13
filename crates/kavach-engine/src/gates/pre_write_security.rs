//! Stage 1: Security checks — path blocking, secrets, python ban, memory-file
//! guard. Returns early on block or allow-with-context (agent/sensitive files).
//!
//! The stages are ordered: empty-path → system-path → bidi → secret → agent →
//! sensitive → python → empty-code-write → memory-file. The first hit wins.
//!
//! `hardcoded_secret` runs BEFORE the agent/sensitive advisory-allow stages: a
//! `.env`/credential file IS a sensitive path, and the sensitive stage returns
//! `AllowEarly` (advisory) — so if it ran first it would short-circuit the secret
//! scan and wave a real `AWS_SECRET_ACCESS_KEY=AKIA…` write through with only a
//! warning. Secrets are a hard block, so they must be evaluated before any
//! allow-early advisory. SOURCE: loophole audit (cursor-edge), runtime-proven.
mod content_stages;
mod path_stages;

#[cfg(test)]
mod tests;

use crate::gates::pre_write_context::WriteContext;

/// Result of the security stage.
pub(crate) enum SecurityResult {
    /// Hard block — deny the write with this reason.
    Block(String),
    /// Allow early with advisory context (agent files, sensitive files).
    AllowEarly(String),
    /// No security issue — continue to next stage.
    Pass,
}

/// Run all security checks against the write context, first hit wins.
pub(crate) fn check(ctx: &WriteContext<'_>) -> SecurityResult {
    path_stages::empty_path(ctx)
        .or_else(|| path_stages::system_path(ctx))
        .or_else(|| path_stages::bidi_unicode(ctx))
        .or_else(|| content_stages::hardcoded_secret(ctx))
        .or_else(|| path_stages::agent_file(ctx))
        .or_else(|| path_stages::sensitive_file(ctx))
        .or_else(|| content_stages::python_ban(ctx))
        .or_else(|| content_stages::empty_code_write(ctx))
        .or_else(|| content_stages::memory_file(ctx))
        .unwrap_or(SecurityResult::Pass)
}

//! Advisory: a cloud/secret-manager CLI is being used to READ a secret value into
//! the terminal (and thus into model context) instead of operating via a runtime
//! script that keeps the value out of context. Complements `env_guard` (which denies
//! .env/printenv reads); this catches the provider-CLI reveal path. Advisory, never a
//! block — the model may have a legitimate non-reveal use. See
//! decision.engine.secret-cli-runtime-script-advisory.
use crate::gates::pre_tool_bash::strip_quoted_regions;
/// Provider secret-manager READ verbs that surface a value to stdout. The `put`/`set`
/// WRITE forms are intentionally excluded — writing a secret is the safe op.
const SECRET_READ_VERBS: &[&str] = &[
    "wrangler secret list",
    "gcloud secrets versions access",
    "aws secretsmanager get-secret-value",
    "az keyvault secret show",
    "vault read",
    "vault kv get",
    "op read",
    "op item get",
    "doppler secrets get",
    "kubectl get secret",
];
/// A reader that would dump the piped/printed value into the terminal — the tell that
/// a value is about to enter context rather than be consumed by a process.
const REVEAL_SINKS: &[&str] = &[
    "| cat", "| bat", "| less", "| head", "| tail", "echo", "printf",
];
/// `Some(advisory)` when the command reads a secret value via a provider CLI AND
/// routes it to a reveal sink (or is a bare read that prints by default). `None`
/// otherwise. Quote-aware: a secret verb inside a quoted string is data, not a call.
pub(crate) fn check_secret_cli_read(command: &str) -> Option<String> {
    let scrubbed = strip_quoted_regions(command);
    let lc = scrubbed.to_lowercase();
    let verb = SECRET_READ_VERBS.iter().find(|v| lc.contains(**v))?;
    // A bare provider read prints to stdout by default → always reveals; a piped form
    // reveals only when the sink is a reader. Either way the value enters context.
    let revealed = REVEAL_SINKS.iter().any(|s| lc.contains(*s)) || !lc.contains('>');
    if !revealed {
        // Redirected to a file with no reader sink — value not surfaced to context.
        return None;
    }
    Some(format!(
        "[ADVISORY:secret-read] `{verb}` surfaces a SECRET VALUE to the terminal, so it \
         enters model context (and the transcript). Do NOT read the value. Instead operate \
         via a runtime script that keeps the value out of context: write a `/tmp/op.sh` that \
         reads the secret into an env var and USES it in the same process, emitting only a \
         non-secret receipt (exit code / id / 'ok') — never the value. Pattern: \
         `printf '%s' \"$(<secret-read-cmd>)\" | <consumer>` inside the script, run it, read \
         the receipt. (global CLAUDE.md §secret-bound runtime script.)"
    ))
}
#[cfg(test)]
#[path = "secret_cli_test.rs"]
#[path = "secret_cli_test.rs"]
mod tests;

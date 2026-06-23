//! Pure per-vendor headless argv builders. Contracts researched 2026-06-24:
//! cc `claude -p` (code.claude.com/docs/en/headless), codex `codex exec`
//! (developers.openai.com/codex/hooks), opencode/gemini non-interactive run.
use super::VendorRequest;

/// `claude -p <prompt> --permission-mode bypassPermissions --max-turns N`.
pub(crate) fn cc_argv(req: &VendorRequest) -> Vec<String> {
    vec![
        "claude".into(),
        "-p".into(),
        req.prompt.clone(),
        "--permission-mode".into(),
        "bypassPermissions".into(),
        "--max-turns".into(),
        req.max_turns.to_string(),
    ]
}

/// `codex exec <prompt>`.
pub(crate) fn codex_argv(req: &VendorRequest) -> Vec<String> {
    vec!["codex".into(), "exec".into(), req.prompt.clone()]
}

/// `opencode run <prompt>` (non-interactive).
pub(crate) fn opencode_argv(req: &VendorRequest) -> Vec<String> {
    vec!["opencode".into(), "run".into(), req.prompt.clone()]
}

/// `gemini -p <prompt>` (non-interactive).
pub(crate) fn gemini_argv(req: &VendorRequest) -> Vec<String> {
    vec!["gemini".into(), "-p".into(), req.prompt.clone()]
}

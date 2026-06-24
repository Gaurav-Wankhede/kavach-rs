//! `kavach lint init` — install the per-language strict-rules profile so the
//! build itself FAILS on bad patterns (no suppression). Language-agnostic:
//! detects Rust/TS/Go by manifest and seeds the canonical strict ruleset.
//! SOURCE: decision.lint.language-profile-template.
pub(crate) mod audit;
pub(crate) mod debt;
pub(crate) mod detect;
pub(crate) mod init;
pub(crate) mod profiles;
pub(crate) mod walk;

/// Dispatch entry for `kavach lint <action>`.
pub(crate) fn run(action: crate::cli::LintAction) -> i32 {
    use crate::cli::LintAction;
    match action {
        LintAction::Init { path, dry_run } => {
            let root = resolve_root(path.as_deref());
            init::run(&root, dry_run)
        }
        LintAction::Audit { path } => audit::run(&resolve_root(path.as_deref())),
        LintAction::Debt { path } => debt::run(&resolve_root(path.as_deref())),
    }
}

/// Resolve the project root: an explicit `--path`, else walk up from cwd to the
/// nearest ancestor with a recognized manifest, else cwd (so init still runs).
fn resolve_root(path: Option<&str>) -> std::path::PathBuf {
    if let Some(p) = path {
        return std::path::PathBuf::from(p);
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut probe = cwd.as_path();
    loop {
        if !detect::detect(probe).is_empty() {
            return probe.to_path_buf();
        }
        match probe.parent() {
            Some(parent) => probe = parent,
            None => return cwd.clone(),
        }
    }
}

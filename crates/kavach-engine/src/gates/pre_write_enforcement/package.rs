//! Stage 7: new-package creation guard. A Write that creates a Cargo.toml in
//! the session workdir requires explicit confirmation (marker / env / session).
use crate::gates::pre_write_context::WriteContext;

/// `Some(reason)` when an unconfirmed new crate would be created in-workdir.
/// Clears the one-shot session confirmation once it has been honoured.
pub(super) fn package_check(
    ctx: &WriteContext<'_>,
    session: &mut kavach_session::SessionState,
) -> Option<String> {
    let pkg_warn = super::super::new_package_guard::check_new_package(ctx.file_path);
    if ctx.tool_name != "Write" || pkg_warn.is_none() {
        return None;
    }
    let in_session_workdir =
        !session.work_dir.is_empty() && ctx.file_path.starts_with(&session.work_dir);
    let env_value = std::env::var("KAVACH_ALLOW_NEW_CRATE").ok();
    let allowed = new_crate_allowed(
        session.new_crate_confirmed,
        env_value.as_deref(),
        ctx.content,
    );
    if in_session_workdir && !allowed {
        return Some(
            "NEW_CRATE_CONFIRMATION_REQUIRED\n\
             \n\
             Add this as the FIRST LINE of the Cargo.toml content:\n\
             # kavach: new-crate confirmed by user\n\
             The gate reads this marker and allows automatically.\n\
             \n\
             FORBIDDEN: permission-seeking phrases, env vars, kavach gates --hook invocations."
                .to_owned(),
        );
    }
    if in_session_workdir && session.new_crate_confirmed {
        session.clear_new_crate_confirmed();
    }
    None
}

/// Check if new crate creation is allowed (session flag / env=1 / inline marker).
pub(super) fn new_crate_allowed(
    session_confirmed: bool,
    env_value: Option<&str>,
    content: &str,
) -> bool {
    session_confirmed
        || matches!(env_value, Some("1"))
        || content.contains("# kavach: new-crate confirmed by user")
}

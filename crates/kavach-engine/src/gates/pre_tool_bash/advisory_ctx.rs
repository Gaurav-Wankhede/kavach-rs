//! Stateful advisory tail: env/prod hard-blocks, loop + duplicate-test guards,
//! then the collected advisory-context parts. Runs only after `blocklist::check`
//! falls through.
use super::advisories::{
    check_commit_message, check_multi_crate, check_nextest_advisory, check_secret_cli_read,
    check_toolbelt_cli,
};
use super::decision::Decision;
use super::test_tracker::{check_duplicate_test_run, register_test_run};

pub(super) fn run(command: &str) -> Decision {
    let grep_ctx = super::super::grep_guard::check_grep_command(command);
    let commit_ctx = check_commit_message(command);
    let check_ctx = check_multi_crate(command);
    let nextest_ctx = check_nextest_advisory(command);
    let secret_ctx = check_secret_cli_read(command);
    let toolbelt_ctx = check_toolbelt_cli(command);

    if let Some(reason) = super::super::env_guard::check_env_value_read(command) {
        return Decision::Deny(reason);
    }
    if let Some(reason) = super::super::prod_guard::check_prod_destructive(command) {
        return Decision::Deny(reason);
    }

    let env_ctx = super::super::env_guard::check_env_sourcing(command);
    let prod_ctx = super::super::prod_guard::check_prod_ops(command);
    let mut session = kavach_session::get_or_create_session();

    if let Some(reason) = super::super::loop_guard::check_bash_loop(&session, command) {
        return Decision::Allow(Some(format!("[ADVISORY:bash-loop] {reason}")));
    }
    super::super::loop_guard::record_command(&mut session, command);

    if let Some(reason) = check_duplicate_test_run(&session, command) {
        return Decision::Allow(Some(format!("[ADVISORY:duplicate-test] {reason}")));
    }
    register_test_run(&mut session, command);

    let scaffold_ctx = std::env::current_dir()
        .ok()
        .and_then(|cwd| super::advisories::scaffold_nextest_config(command, &cwd));
    let module_ctx = session.inject_modules_once(&["commands"]);
    let parts: Vec<&str> = [
        Some(module_ctx.as_str()),
        scaffold_ctx.as_deref(),
        grep_ctx.as_deref(),
        check_ctx.as_deref(),
        commit_ctx.as_deref(),
        nextest_ctx.as_deref(),
        secret_ctx.as_deref(),
        toolbelt_ctx.as_deref(),
        env_ctx.as_deref(),
        prod_ctx.as_deref(),
    ]
    .into_iter()
    .flatten()
    .filter(|s| !s.is_empty())
    .collect();

    if parts.is_empty() {
        Decision::Allow(None)
    } else {
        Decision::Allow(Some(parts.join("\n\n")))
    }
}

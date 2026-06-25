mod lock;
mod build;
mod install;

use std::path::PathBuf;
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code};

pub(crate) fn run(skip_tests: bool) -> i32 {
    let Some(root) = workspace_root() else {
        if let Err(io_err) =
            ewrite_or_exit("[DEPLOY] FAIL: cannot resolve workspace root for the deploy lock.")
        {
            return into_exit_code(io_err);
        }
        return 1;
    };
    let _deploy_lock = match lock::DeployLock::try_acquire(&root) {
        Ok(Some(guard)) => guard,
        Ok(None) => {
            if let Err(io_err) = ewrite_or_exit(
                "[DEPLOY] FAIL: another `kavach deploy` is already running (holds .deploy.lock). \
                 Refusing to race the binary install + daemon restart. Re-run once it finishes.",
            ) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        Err(fs_err) => {
            if let Err(io_err) = ewrite_or_exit(&format!(
                "[DEPLOY] FAIL: cannot open the deploy lock ({}): {fs_err}",
                lock::DEPLOY_LOCK_NAME
            )) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };

    build::deploy_cli(skip_tests)
}

fn workspace_root() -> Option<PathBuf> {
    std::env::current_dir().ok()
}

#[cfg(all(test, unix))]
#[path = "deploy/tests.rs"]
mod tests;

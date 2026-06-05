//! Env-value-leak patterns, grouped: environment-variable reads (`env_vars`)
//! and `.env`-file reads (`dotenv`). The hub re-exports each `check_*` so the
//! parent gate can chain them.
mod dotenv;
mod env_vars;
mod util;

pub(super) use dotenv::{check_dotenv_grep, check_dotenv_read, check_source};
pub(super) use env_vars::{
    check_echo, check_env_grep, check_printenv, check_proc_environ, check_python_environ,
    check_set_dump,
};

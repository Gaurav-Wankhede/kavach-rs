// kavach rpc subcommand — bridges CLI to kavach_rpc::run.
// CRITICAL: in stdio mode, stdout is the JSON-RPC protocol channel — caller
// must ensure no `println!` runs before kavach_rpc::run takes over.
use kavach_rpc::TransportKind;

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code};

pub(super) fn run(transport: &str, apply_schema: bool) -> i32 {
    let kind = match transport {
        "stdio" => TransportKind::Stdio,
        "http" => TransportKind::Http,
        #[cfg(unix)]
        "unix" => TransportKind::Unix,
        other => {
            let msg =
                format!("kavach rpc: unknown transport '{other}' (expected: stdio, http, unix)");
            if let Err(e) = ewrite_or_exit(&msg) {
                return into_exit_code(e);
            }
            return 2;
        }
    };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("kavach rpc: tokio runtime: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };

    if matches!(kind, TransportKind::Http) {
        let _init_result = tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_env("KAVACH_RPC_LOG")
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .try_init();
    }

    match runtime.block_on(kavach_rpc::run(kind, apply_schema)) {
        Ok(()) => 0,
        Err(e) => {
            let msg = format!("kavach rpc: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            1
        }
    }
}

use kavach_rpc::client::ClientError;

pub(super) const DAEMON_UNAVAILABLE: &str = "daemon_unavailable";

pub(super) fn format_err(e: ClientError) -> String {
    match e {
        ClientError::NotReachable(_) => DAEMON_UNAVAILABLE.to_owned(),
        ClientError::Io(io_err) => format!("io: {io_err}"),
        ClientError::Json(json_err) => format!("json: {json_err}"),
        ClientError::Rpc { code, message } => format!("rpc[{code}]: {message}"),
        ClientError::NoResult => "no_result".to_owned(),
    }
}

pub(super) fn should_fallback_to_direct(rpc_err: &str) -> bool {
    rpc_err == DAEMON_UNAVAILABLE
}

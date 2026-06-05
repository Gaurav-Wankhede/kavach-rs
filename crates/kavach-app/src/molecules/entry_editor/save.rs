// SOURCE: https://docs.rs/jsonrpsee (jsonrpsee is async-only; kavach-rpc ships a hand-rolled sync UDS client for hook hot paths)
use serde::{Deserialize, Serialize};

use crate::rpc_client::{Error as RpcError, rpc};
use crate::state::EntryRef;

#[derive(Debug, Serialize)]
struct WriteParams<'a> {
    project: &'a str,
    category: &'a str,
    key: &'a str,
    title: &'a str,
    content: Option<&'a str>,
    new: bool,
    update_key: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct StatusUpdateParams<'a> {
    project: &'a str,
    category: &'a str,
    key: &'a str,
    status: &'a str,
}

#[derive(Debug, Deserialize)]
struct OkOrErrDto {
    success: bool,
    #[serde(default)]
    error: Option<String>,
}

fn err_message(err: Option<String>, fallback: &str) -> String {
    err.unwrap_or_else(|| fallback.to_owned())
}

fn map_err(e: RpcError) -> String {
    match e {
        RpcError::DaemonOffline(_) => {
            "kavach-rpc daemon offline — start via `kavach rpc serve`".to_owned()
        }
        RpcError::Rpc { code, message } => format!("rpc {code}: {message}"),
        RpcError::Io(s) | RpcError::Decode(s) => s,
    }
}

pub fn save(target: &EntryRef) -> Result<(), String> {
    let res_write: OkOrErrDto = rpc(
        "db.write",
        WriteParams {
            project: &target.project_slug,
            category: &target.category,
            key: &target.key,
            title: &target.title,
            content: Some(&target.content),
            new: false,
            update_key: Some(&target.key),
        },
    )
    .map_err(map_err)?;
    if !res_write.success {
        return Err(err_message(res_write.error, "db.write returned !success"));
    }

    let res_status: OkOrErrDto = rpc(
        "db.status_update",
        StatusUpdateParams {
            project: &target.project_slug,
            category: &target.category,
            key: &target.key,
            status: target.status.as_ref(),
        },
    )
    .map_err(map_err)?;
    if !res_status.success {
        return Err(err_message(
            res_status.error,
            "db.status_update returned !success",
        ));
    }
    Ok(())
}

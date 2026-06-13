//! `kavach db gate-config {get,set,delete,list}` — operator door to the dynamic
//! gate-config overlay over the live RPC daemon. Lets a value be retuned per
//! project at runtime without a rebuild; absence falls through to the gate's
//! compiled default (fail-closed).
use crate::cmd::db::rpc_client;
use kavach_rpc::methods::db::GateValueDto;

/// Resolve and print one override, or report the miss (caller uses its default).
pub(super) fn get(project: &str, gate_key: &str) -> i32 {
    match rpc_client::gate_config_get(project, gate_key) {
        Ok(Some(v)) => {
            println!("{}", render_value(&v));
            0
        }
        Ok(None) => {
            println!("(no override — gate uses its compiled default)");
            0
        }
        Err(e) => emit_err(&format!("gate-config get: {e}")),
    }
}

/// Build the typed DTO from the CLI flags (one value per kind) and upsert it.
pub(super) fn set(
    project: &str,
    gate_key: &str,
    kind: &str,
    num: Option<f64>,
    boolean: Option<bool>,
    list: Option<String>,
    text: Option<String>,
) -> i32 {
    // Validate at the edge: the value flag must match the kind, so a malformed
    // invocation is rejected here rather than silently dropping a column.
    let dto = match kind {
        "threshold" => match num {
            Some(n) => dto(kind, Some(n), None, None, None),
            None => return emit_err("gate-config set: --kind threshold needs --num"),
        },
        "enabled" => match boolean {
            Some(b) => dto(kind, None, Some(b), None, None),
            None => return emit_err("gate-config set: --kind enabled needs --boolean"),
        },
        "pattern_list" => match list {
            Some(l) => {
                let items: Vec<String> = l.split(',').map(str::trim).map(str::to_owned).collect();
                dto(kind, None, None, Some(items), None)
            }
            None => return emit_err("gate-config set: --kind pattern_list needs --list a,b,c"),
        },
        "severity" | "text" => match text {
            Some(t) => dto(kind, None, None, None, Some(t)),
            None => return emit_err("gate-config set: --kind severity|text needs --text"),
        },
        other => return emit_err(&format!("gate-config set: unknown --kind {other}")),
    };
    match rpc_client::gate_config_set(project, gate_key, dto) {
        Ok(_) => {
            println!("ok — override set for {project}/{gate_key}");
            0
        }
        Err(e) => emit_err(&format!("gate-config set: {e}")),
    }
}

/// Remove an override; the gate reverts to its compiled default. Idempotent.
pub(super) fn delete(project: &str, gate_key: &str) -> i32 {
    match rpc_client::gate_config_delete(project, gate_key) {
        Ok(_) => {
            println!("ok — override removed for {project}/{gate_key}");
            0
        }
        Err(e) => emit_err(&format!("gate-config delete: {e}")),
    }
}

/// List every override for a project.
pub(super) fn list(project: &str) -> i32 {
    match rpc_client::gate_config_list(project) {
        Ok(rows) if rows.is_empty() => {
            println!("(no overrides for {project} — all gates use compiled defaults)");
            0
        }
        Ok(rows) => {
            for r in rows {
                println!("{}\t{}\t{}", r.project, r.gate_key, r.kind);
            }
            0
        }
        Err(e) => emit_err(&format!("gate-config list: {e}")),
    }
}

fn dto(
    kind: &str,
    num: Option<f64>,
    boolean: Option<bool>,
    list: Option<Vec<String>>,
    text: Option<String>,
) -> GateValueDto {
    GateValueDto {
        kind: kind.to_owned(),
        num,
        boolean,
        list,
        text,
    }
}

fn render_value(v: &GateValueDto) -> String {
    match (v.num, v.boolean, &v.list, &v.text) {
        (Some(n), _, _, _) => format!("{}: {n}", v.kind),
        (_, Some(b), _, _) => format!("{}: {b}", v.kind),
        (_, _, Some(l), _) => format!("{}: {}", v.kind, l.join(", ")),
        (_, _, _, Some(t)) => format!("{}: {t}", v.kind),
        _ => format!("{}: (empty)", v.kind),
    }
}

fn emit_err(msg: &str) -> i32 {
    eprintln!("{msg}");
    1
}

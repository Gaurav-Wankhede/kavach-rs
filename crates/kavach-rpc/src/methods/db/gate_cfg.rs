//! `db.gate_config_*` RPC verbs — the daemon-side door to the gate-config
//! overlay store (`unit.dynamic-gate-config-plane` P2). A gate resolves a
//! dynamic value through `kavach-config::gate_value`, which calls
//! `db.gate_config_get` here; absence falls through to the compiled default.
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::{
    GateConfigEntry, GateConfigKind, GateConfigValue, gate_config_delete, gate_config_list,
    gate_config_resolve, gate_config_set_with_kind,
};
use serde::{Deserialize, Serialize};

// The wire DTO is defined once in `kavach-types` (a leaf crate both the engine
// and the pattern detectors can reach) and re-exported here so existing
// `kavach_rpc::methods::db::GateValueDto` paths keep resolving unchanged. The
// `into_typed`/`from_value` conversions stay here because they touch the
// surreal-side `GateConfigValue`/`GateConfigKind`, which the leaf crate must not
// depend on.
pub use kavach_types::GateValueDto;

/// Project the typed `(value, kind)` from the wire shape, or `None` if the kind
/// tag is unknown or its value field is absent (fail-closed: a garbled DTO
/// resolves to "no override"). A free fn (not a method) because `GateValueDto`
/// is now a foreign type owned by `kavach-types`.
fn dto_into_typed(d: GateValueDto) -> Option<(GateConfigValue, GateConfigKind)> {
    match d.kind.as_str() {
        "threshold" => d
            .num
            .map(|n| (GateConfigValue::Threshold(n), GateConfigKind::Threshold)),
        "pattern_list" => d
            .list
            .map(|l| (GateConfigValue::PatternList(l), GateConfigKind::PatternList)),
        "enabled" => d
            .boolean
            .map(|b| (GateConfigValue::Enabled(b), GateConfigKind::Enabled)),
        "severity" => d
            .text
            .map(|t| (GateConfigValue::Text(t), GateConfigKind::Severity)),
        "text" => d
            .text
            .map(|t| (GateConfigValue::Text(t), GateConfigKind::Text)),
        _ => None,
    }
}

/// Build the wire shape from a resolved value (kind derived from the variant;
/// `Text` reports as `text`, severities are indistinguishable on read which is
/// fine — both are string-shaped to the caller).
fn dto_from_value(v: &GateConfigValue) -> GateValueDto {
    let base = GateValueDto {
        kind: String::new(),
        num: None,
        boolean: None,
        list: None,
        text: None,
    };
    match v {
        GateConfigValue::Threshold(n) => GateValueDto {
            kind: "threshold".to_owned(),
            num: Some(*n),
            ..base
        },
        GateConfigValue::PatternList(l) => GateValueDto {
            kind: "pattern_list".to_owned(),
            list: Some(l.clone()),
            ..base
        },
        GateConfigValue::Enabled(b) => GateValueDto {
            kind: "enabled".to_owned(),
            boolean: Some(*b),
            ..base
        },
        GateConfigValue::Text(t) => GateValueDto {
            kind: "text".to_owned(),
            text: Some(t.clone()),
            ..base
        },
        // `GateConfigValue` is #[non_exhaustive] across the crate boundary; a
        // future variant resolves to the empty (unknown-kind) DTO, which
        // `dto_into_typed` reads back as `None` — fail-closed, never a panic.
        _ => base,
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct GetParams {
    pub project: String,
    pub gate_key: String,
}

/// Resolve `(project, gate_key)` with project-then-global fallback. `None` when
/// no override exists (caller uses its compiled default).
///
/// # Errors
/// Returns `ErrorObjectOwned` when the database query fails.
pub async fn get(state: &AppState, p: GetParams) -> Result<Option<GateValueDto>, ErrorObjectOwned> {
    let resolved = gate_config_resolve(&state.db, &p.project, &p.gate_key)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(resolved.as_ref().map(dto_from_value))
}

#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct SetParams {
    pub project: String,
    pub gate_key: String,
    pub value: GateValueDto,
}

/// Upsert a gate-config override.
///
/// # Errors
/// Returns `ErrorObjectOwned` when the DTO is malformed (unknown kind / missing
/// value) or the write fails (including a kind/value shape mismatch).
pub async fn set(state: &AppState, p: SetParams) -> Result<&'static str, ErrorObjectOwned> {
    let Some((value, kind)) = dto_into_typed(p.value) else {
        return Err(ErrorObjectOwned::owned(
            -32602,
            "gate_config.set: malformed value DTO (unknown kind or missing value field)",
            None::<()>,
        ));
    };
    gate_config_set_with_kind(&state.db, &p.project, &p.gate_key, &value, kind)
        .await
        .map_err(surreal_to_rpc)?;
    Ok("ok")
}

#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct DeleteParams {
    pub project: String,
    pub gate_key: String,
}

/// Remove an override, reverting the gate to its file/compiled default.
/// Idempotent — deleting an absent key succeeds.
///
/// # Errors
/// Returns `ErrorObjectOwned` when the database delete fails.
pub async fn delete(state: &AppState, p: DeleteParams) -> Result<&'static str, ErrorObjectOwned> {
    gate_config_delete(&state.db, &p.project, &p.gate_key)
        .await
        .map_err(surreal_to_rpc)?;
    Ok("ok")
}

#[derive(Debug, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundary"
)]
pub struct ListParams {
    pub project: String,
}

/// List every override for a project (`*` for the globals).
///
/// # Errors
/// Returns `ErrorObjectOwned` when the database query fails.
pub async fn list(
    state: &AppState,
    p: ListParams,
) -> Result<Vec<GateConfigEntry>, ErrorObjectOwned> {
    gate_config_list(&state.db, &p.project)
        .await
        .map_err(surreal_to_rpc)
}

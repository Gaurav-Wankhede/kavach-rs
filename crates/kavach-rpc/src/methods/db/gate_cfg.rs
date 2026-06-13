//! `db.gate_config_*` RPC verbs — the daemon-side door to the gate-config
//! overlay store (`unit.dynamic-gate-config-plane` P2). A gate resolves a
//! dynamic value through `kavach-config::gate_value`, which calls
//! `db.gate_config_get` here; absence falls through to the compiled default.
use crate::error::surreal_to_rpc;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use kavach_surreal::{
    GateConfigEntry, GateConfigKind, GateConfigValue, gate_config_list, gate_config_resolve,
    gate_config_set_with_kind,
};
use serde::{Deserialize, Serialize};

/// Wire shape for a gate-config value: a kind tag plus the one populated value
/// field. Flat (not a Rust enum) so it serializes cleanly over JSON-RPC and is
/// trivial to construct from any client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler + client boundary"
)]
pub struct GateValueDto {
    /// `threshold` | `pattern_list` | `enabled` | `severity` | `text`.
    pub kind: String,
    #[serde(default)]
    pub num: Option<f64>,
    #[serde(default)]
    pub boolean: Option<bool>,
    #[serde(default)]
    pub list: Option<Vec<String>>,
    #[serde(default)]
    pub text: Option<String>,
}

impl GateValueDto {
    /// Project the typed `(value, kind)` from the wire shape, or `None` if the
    /// kind tag is unknown or its value field is absent (fail-closed: a garbled
    /// DTO resolves to "no override").
    fn into_typed(self) -> Option<(GateConfigValue, GateConfigKind)> {
        match self.kind.as_str() {
            "threshold" => self
                .num
                .map(|n| (GateConfigValue::Threshold(n), GateConfigKind::Threshold)),
            "pattern_list" => self
                .list
                .map(|l| (GateConfigValue::PatternList(l), GateConfigKind::PatternList)),
            "enabled" => self
                .boolean
                .map(|b| (GateConfigValue::Enabled(b), GateConfigKind::Enabled)),
            "severity" => self
                .text
                .map(|t| (GateConfigValue::Text(t), GateConfigKind::Severity)),
            "text" => self
                .text
                .map(|t| (GateConfigValue::Text(t), GateConfigKind::Text)),
            _ => None,
        }
    }

    /// Build the wire shape from a resolved value (kind derived from the
    /// variant; `Text` reports as `text`, severities are indistinguishable on
    /// read which is fine — both are string-shaped to the caller).
    fn from_value(v: &GateConfigValue) -> Self {
        let base = Self {
            kind: String::new(),
            num: None,
            boolean: None,
            list: None,
            text: None,
        };
        match v {
            GateConfigValue::Threshold(n) => Self {
                kind: "threshold".to_owned(),
                num: Some(*n),
                ..base
            },
            GateConfigValue::PatternList(l) => Self {
                kind: "pattern_list".to_owned(),
                list: Some(l.clone()),
                ..base
            },
            GateConfigValue::Enabled(b) => Self {
                kind: "enabled".to_owned(),
                boolean: Some(*b),
                ..base
            },
            GateConfigValue::Text(t) => Self {
                kind: "text".to_owned(),
                text: Some(t.clone()),
                ..base
            },
            // `GateConfigValue` is #[non_exhaustive] across the crate boundary;
            // a future variant resolves to the empty (unknown-kind) DTO, which
            // `into_typed` reads back as `None` — fail-closed, never a panic.
            _ => base,
        }
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
pub async fn get(
    state: &AppState,
    p: GetParams,
) -> Result<Option<GateValueDto>, ErrorObjectOwned> {
    let resolved = gate_config_resolve(&state.db, &p.project, &p.gate_key)
        .await
        .map_err(surreal_to_rpc)?;
    Ok(resolved.as_ref().map(GateValueDto::from_value))
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
    let Some((value, kind)) = p.value.into_typed() else {
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

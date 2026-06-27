//! Dynamic gate-config overlay store (unit.dynamic-gate-config-plane P1).
//!
//! The DB layer of the resolver chain `DB > file > compiled-default`. A row
//! OVERRIDES a gate constant at runtime; absence falls through to the
//! file/compiled default (fail-closed — a missing row never disables a gate).
//!
//! Value is discriminated by [`GateConfigKind`]: exactly one `value_*` column is
//! populated, enforced at the write edge so a threshold can never hold a string
//! and a pattern-list can never hold a number (illegal cross-kind states are
//! unrepresentable past `set`). `project = "*"` is the global row.
use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::SurrealValue;
use crate::error::{Error, Result};
/// The global (project-agnostic) row sentinel. A per-project lookup falls back
/// to this when no project-scoped row exists.
pub const GLOBAL_PROJECT: &str = "*";
/// The kind tag that selects which `value_*` column carries the payload. Mirrors
/// the `ASSERT` on `gate_config.kind` in `schema.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, SurrealValue)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum GateConfigKind {
    /// A numeric tunable (similarity cutoff, byte cap, TTL, z-score).
    Threshold,
    /// An additive detection-pattern / safelist list.
    PatternList,
    /// A gate enablement toggle.
    Enabled,
    /// A gate severity override (`p0` / `p1` / `advisory` as text).
    Severity,
    /// Injected context text (the autonomy contract, advisory copy).
    Text,
}
impl GateConfigKind {
    /// The bare wire string stored in the SCHEMAFULL `gate_config.kind` column.
    /// MUST be bound as this `&str` (not the enum) — a `SurrealValue`-derived
    /// fieldless enum serializes as a tagged object, which fails the column's
    /// `TYPE string ASSERT` and silently drops the CREATE.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Threshold => "threshold",
            Self::PatternList => "pattern_list",
            Self::Enabled => "enabled",
            Self::Severity => "severity",
            Self::Text => "text",
        }
    }
    /// Parse the bare wire string back to the kind; `None` on an unknown tag
    /// (fail-closed: a corrupt `kind` reads as no-override).
    #[must_use]
    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "threshold" => Some(Self::Threshold),
            "pattern_list" => Some(Self::PatternList),
            "enabled" => Some(Self::Enabled),
            "severity" => Some(Self::Severity),
            "text" => Some(Self::Text),
            _ => None,
        }
    }
}
/// A resolved gate-config value — the discriminated union the resolver returns.
/// Exactly one variant is produced per row, matching its [`GateConfigKind`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum GateConfigValue {
    /// `kind = threshold`.
    Threshold(f64),
    /// `kind = pattern_list`.
    PatternList(Vec<String>),
    /// `kind = enabled`.
    Enabled(bool),
    /// `kind = severity` or `kind = text` (both string-shaped).
    Text(String),
}
/// Raw row as stored — the four optional value columns + the kind tag. Only the
/// column matching `kind` is ever `Some` after a validated `set`.
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
struct GateConfigRow {
    project: String,
    gate_key: String,
    // `kind` is the BARE wire string (`threshold`/…), NOT the enum: the
    // SCHEMAFULL column is `TYPE string`, and a `SurrealValue`-derived fieldless
    // enum round-trips as a tagged object that neither writes nor reads against
    // a string column. Parsed to the typed kind in `value()` via `from_wire`.
    kind: String,
    #[serde(default)]
    value_num: Option<f64>,
    #[serde(default)]
    value_bool: Option<bool>,
    #[serde(default)]
    value_list: Option<Vec<String>>,
    #[serde(default)]
    value_text: Option<String>,
}
impl GateConfigRow {
    /// Project the populated value column for this row's `kind`. Returns `None`
    /// on an unknown kind OR a shape mismatch (the wrong column populated) —
    /// fail-closed: a corrupt row reads as "no override", so the resolver falls
    /// through to the compiled default rather than feeding a gate a garbage value.
    fn value(&self) -> Option<GateConfigValue> {
        match GateConfigKind::from_wire(&self.kind)? {
            GateConfigKind::Threshold => self.value_num.map(GateConfigValue::Threshold),
            GateConfigKind::PatternList => {
                self.value_list.clone().map(GateConfigValue::PatternList)
            }
            GateConfigKind::Enabled => self.value_bool.map(GateConfigValue::Enabled),
            GateConfigKind::Severity | GateConfigKind::Text => {
                self.value_text.clone().map(GateConfigValue::Text)
            }
        }
    }
}
/// The four value columns for one row — exactly one is `Some`, matching the
/// row's kind. Built by [`Columns::for_value`] so column population happens in a
/// single place and a `set` can never write two columns.
struct Columns<'a> {
    num: Option<f64>,
    boolean: Option<bool>,
    list: Option<&'a Vec<String>>,
    text: Option<&'a String>,
}
impl<'a> Columns<'a> {
    const fn for_value(value: &'a GateConfigValue) -> Self {
        match value {
            GateConfigValue::Threshold(n) => Self {
                num: Some(*n),
                boolean: None,
                list: None,
                text: None,
            },
            GateConfigValue::Enabled(b) => Self {
                num: None,
                boolean: Some(*b),
                list: None,
                text: None,
            },
            GateConfigValue::PatternList(l) => Self {
                num: None,
                boolean: None,
                list: Some(l),
                text: None,
            },
            GateConfigValue::Text(t) => Self {
                num: None,
                boolean: None,
                list: None,
                text: Some(t),
            },
        }
    }
}
/// The `kind` tag a value carries — used to stamp the row so reads project the
/// right column. `Text` maps to `Text`; `Severity` is written by the caller via
/// [`set_with_kind`] when the string is a severity rather than free text.
const fn kind_of(value: &GateConfigValue) -> GateConfigKind {
    match value {
        GateConfigValue::Threshold(_) => GateConfigKind::Threshold,
        GateConfigValue::PatternList(_) => GateConfigKind::PatternList,
        GateConfigValue::Enabled(_) => GateConfigKind::Enabled,
        GateConfigValue::Text(_) => GateConfigKind::Text,
    }
}
/// Fetch the override for `(project, gate_key)`, or `None` when absent.
///
/// One exact lookup — the caller orders any project-then-global fallback.
///
/// # Errors
/// Propagates `Error::Surreal` when the query fails.
pub async fn gate_config_get(
    db: &Surreal<Db>,
    project: &str,
    gate_key: &str,
) -> Result<Option<GateConfigValue>> {
    let row: Option<GateConfigRow> = db
        .query(
            "SELECT * FROM gate_config \
             WHERE project = $p AND gate_key = $k LIMIT 1",
        )
        .bind(("p", project.to_owned()))
        .bind(("k", gate_key.to_owned()))
        .await
        .map_err(Error::Surreal)?
        .take(0)
        .map_err(Error::Surreal)?;
    Ok(row.and_then(|r| r.value()))
}
/// Resolve `(project, gate_key)` with project-then-global fallback.
///
/// A project-scoped row wins; absent that, the global `*` row; absent both,
/// `None` (caller falls through to file/compiled default).
///
/// # Errors
/// Propagates `Error::Surreal` when either query fails.
pub async fn gate_config_resolve(
    db: &Surreal<Db>,
    project: &str,
    gate_key: &str,
) -> Result<Option<GateConfigValue>> {
    if project != GLOBAL_PROJECT
        && let Some(scoped) = gate_config_get(db, project, gate_key).await?
    {
        return Ok(Some(scoped));
    }
    gate_config_get(db, GLOBAL_PROJECT, gate_key).await
}
/// Upsert an override, idempotent on `(project, gate_key)`.
///
/// DELETE-then-CREATE keeps exactly one row per key (last write wins), so a
/// concurrent double-set converges instead of duplicating. The stamped `kind`
/// comes from the value; a severity string must use [`set_with_kind`] so it
/// reads back as `Severity`.
///
/// # Errors
/// Propagates `Error::Surreal` when the write fails.
pub async fn gate_config_set(
    db: &Surreal<Db>,
    project: &str,
    gate_key: &str,
    value: &GateConfigValue,
) -> Result<()> {
    set_with_kind(db, project, gate_key, value, kind_of(value)).await
}
/// As [`gate_config_set`] but with an explicit `kind` stamp.
///
/// The only way to store a `Severity` (whose value is `Text`-shaped). Validates
/// kind matches value shape — fail-closed: a `Threshold` kind with a `Text`
/// value is rejected before it can poison a reader.
///
/// # Errors
/// `Error::Migration` on a kind/value shape mismatch; `Error::Surreal` on write
/// failure.
pub async fn set_with_kind(
    db: &Surreal<Db>,
    project: &str,
    gate_key: &str,
    value: &GateConfigValue,
    kind: GateConfigKind,
) -> Result<()> {
    let shape_ok = matches!(
        (kind, value),
        (GateConfigKind::Threshold, GateConfigValue::Threshold(_))
            | (GateConfigKind::PatternList, GateConfigValue::PatternList(_))
            | (GateConfigKind::Enabled, GateConfigValue::Enabled(_))
            | (
                GateConfigKind::Severity | GateConfigKind::Text,
                GateConfigValue::Text(_)
            )
    );
    if !shape_ok {
        return Err(Error::Migration(format!(
            "gate_config: kind {kind:?} does not match value shape for {gate_key}"
        )));
    }
    let cols = Columns::for_value(value);
    // Single-statement keyed UPSERT, NOT racy DELETE;CREATE (two concurrent
    // setters double-write / a reader sees zero rows mid-swap).
    // SOURCE: decision.algo-upsert-idempotent-keyed.
    let key = crate::decisions::hash_keyed("gate_config", project, gate_key, "");
    db.query(
        "UPSERT type::record('gate_config', $key) SET \
             project = $p, gate_key = $k, kind = $kind, \
             value_num = $num, value_bool = $boolean, \
             value_list = $list, value_text = $text, updated_at = time::now();",
    )
    .bind(("key", key))
    .bind(("p", project.to_owned()))
    .bind(("k", gate_key.to_owned()))
    .bind(("kind", kind.as_str().to_owned()))
    .bind(("num", cols.num))
    .bind(("boolean", cols.boolean))
    .bind(("list", cols.list.cloned()))
    .bind(("text", cols.text.cloned()))
    .await
    .map_err(Error::Surreal)?;
    Ok(())
}
/// Remove the override for `(project, gate_key)`.
///
/// Reverts the gate to its file/compiled default. Idempotent: deleting an absent
/// key is a no-op success (the post-condition — "no override exists" — holds).
///
/// # Errors
/// Propagates `Error::Surreal` when the delete fails.
pub async fn gate_config_delete(db: &Surreal<Db>, project: &str, gate_key: &str) -> Result<()> {
    db.query("DELETE gate_config WHERE project = $p AND gate_key = $k")
        .bind(("p", project.to_owned()))
        .bind(("k", gate_key.to_owned()))
        .await
        .map_err(Error::Surreal)?;
    Ok(())
}
/// One overridden key's identity, for `list`/inspection.
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[non_exhaustive]
pub struct GateConfigEntry {
    /// Owning project (`*` = global).
    pub project: String,
    /// The gate-config key.
    pub gate_key: String,
    /// The discriminator, as the bare wire string (`threshold`/`pattern_list`/
    /// `enabled`/`severity`/`text`) — matches the `TYPE string` DB column.
    pub kind: String,
}
/// List every override for `project` (pass [`GLOBAL_PROJECT`] for the globals).
///
/// # Errors
/// Propagates `Error::Surreal` when the query fails.
pub async fn gate_config_list(db: &Surreal<Db>, project: &str) -> Result<Vec<GateConfigEntry>> {
    let rows: Vec<GateConfigEntry> = db
        .query("SELECT project, gate_key, kind FROM gate_config WHERE project = $p")
        .bind(("p", project.to_owned()))
        .await
        .map_err(Error::Surreal)?
        .take(0)
        .map_err(Error::Surreal)?;
    Ok(rows)
}
#[cfg(test)]
#[path = "gate_config_test.rs"]
mod tests;

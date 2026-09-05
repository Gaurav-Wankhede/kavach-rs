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
use crate::error::{Error, Result};
use crate::turso::TursoStore;
use serde::{Deserialize, Serialize};
/// The global (project-agnostic) row sentinel. A per-project lookup falls back
/// to this when no project-scoped row exists.
pub const GLOBAL_PROJECT: &str = "*";
/// The kind tag that selects which `value_*` column carries the payload. Mirrors
/// the `ASSERT` on `gate_config.kind` in `schema.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    /// The bare wire string stored in the `gate_config.kind` column.
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
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GateConfigRow {
    project: String,
    gate_key: String,
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
/// Map a `gate_config` row (column order: project, gate_key, kind, value_num,
/// value_bool, value_list, value_text) into a [`GateConfigRow`].
fn row_from_row(row: &libsql::Row) -> Result<GateConfigRow> {
    let value_list = row
        .get::<Option<String>>(5)?
        .map(|s| serde_json::from_str(&s).unwrap_or_default())
        .unwrap_or_default();
    Ok(GateConfigRow {
        project: row.get::<String>(0)?,
        gate_key: row.get::<String>(1)?,
        kind: row.get::<String>(2)?,
        value_num: row.get::<Option<f64>>(3)?,
        value_bool: row.get::<Option<bool>>(4)?,
        value_list,
        value_text: row.get::<Option<String>>(6)?,
    })
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
/// Propagates `Error::Turso` when the query fails.
pub async fn gate_config_get(
    db: &TursoStore,
    project: &str,
    gate_key: &str,
) -> Result<Option<GateConfigValue>> {
    let mut rows = db
        .exec_params(
            "SELECT project, gate_key, kind, value_num, value_bool, value_list, value_text \
             FROM gate_config WHERE project = $p AND gate_key = $k LIMIT 1",
            libsql::params![project.to_owned(), gate_key.to_owned()],
        )
        .await?;
    let row = match rows.next().await? {
        Some(row) => row_from_row(&row)?,
        None => return Ok(None),
    };
    Ok(row.value())
}
/// Resolve `(project, gate_key)` with project-then-global fallback.
///
/// A project-scoped row wins; absent that, the global `*` row; absent both,
/// `None` (caller falls through to file/compiled default).
///
/// # Errors
/// Propagates `Error::Turso` when either query fails.
pub async fn gate_config_resolve(
    db: &TursoStore,
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
/// Single-statement keyed UPSERT keeps exactly one row per key (last write
/// wins), so a concurrent double-set converges instead of duplicating. The
/// stamped `kind` comes from the value; a severity string must use
/// [`set_with_kind`] so it reads back as `Severity`.
///
/// # Errors
/// Propagates `Error::Turso` when the write fails.
pub async fn gate_config_set(
    db: &TursoStore,
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
/// `Error::Migration` on a kind/value shape mismatch; `Error::Turso` on write
/// failure.
pub async fn set_with_kind(
    db: &TursoStore,
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
    let list_json = cols.list.map(|l| serde_json::to_string(l)).transpose()?;
    let now = crate::turso::now_rfc3339();
    db.exec_params(
        "INSERT INTO gate_config \
             (project, gate_key, kind, value_num, value_bool, value_list, value_text, updated_at) \
         VALUES ($p, $k, $kind, $num, $boolean, $list, $text, $now) \
         ON CONFLICT(project, gate_key) DO UPDATE SET \
             kind = excluded.kind, value_num = excluded.value_num, \
             value_bool = excluded.value_bool, value_list = excluded.value_list, \
             value_text = excluded.value_text, updated_at = excluded.updated_at",
        libsql::params![
            project.to_owned(),
            gate_key.to_owned(),
            kind.as_str().to_owned(),
            cols.num,
            cols.boolean,
            list_json,
            cols.text.cloned(),
            now
        ],
    )
    .await?;
    Ok(())
}
/// Remove the override for `(project, gate_key)`.
///
/// Reverts the gate to its file/compiled default. Idempotent: deleting an absent
/// key is a no-op success (the post-condition — "no override exists" — holds).
///
/// # Errors
/// Propagates `Error::Turso` when the delete fails.
pub async fn gate_config_delete(db: &TursoStore, project: &str, gate_key: &str) -> Result<()> {
    db.exec_params(
        "DELETE FROM gate_config WHERE project = $p AND gate_key = $k",
        libsql::params![project.to_owned(), gate_key.to_owned()],
    )
    .await?;
    Ok(())
}
/// One overridden key's identity, for `list`/inspection.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
/// Propagates `Error::Turso` when the query fails.
pub async fn gate_config_list(db: &TursoStore, project: &str) -> Result<Vec<GateConfigEntry>> {
    let mut rows = db
        .exec_params(
            "SELECT project, gate_key, kind FROM gate_config WHERE project = $p",
            libsql::params![project.to_owned()],
        )
        .await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push(GateConfigEntry {
            project: row.get::<String>(0)?,
            gate_key: row.get::<String>(1)?,
            kind: row.get::<String>(2)?,
        });
    }
    Ok(out)
}
#[cfg(test)]
#[path = "gate_config_test.rs"]
mod tests;

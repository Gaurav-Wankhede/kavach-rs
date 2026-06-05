// split: intentional — Layer-A RLVR bandit_log store (harness-rl); kept out of
// the generic write.rs so the self-improving-gate subsystem owns its own file.
// sql-safe: queries use static literals + .bind() for params, no user input concat.
//! `bandit_log` persistence — the RLVR reward substrate (harness-rl Waves P2/P3a).
//!
//! Append-only, content-addressed decision rows `(context x, action a,
//! propensity p, reward r)`; `r` is logged `None` at decision time and
//! back-filled DOWNSTREAM once the 3-witness verify resolves. This crate stays
//! decoupled from `kavach-patterns`: the typed `BanditRow` is serialized by the
//! caller and stored opaquely as JSON.

use crate::error::Result;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb_types::{RecordId, SurrealValue};

#[cfg(test)]
#[path = "bandit_test.rs"]
mod tests;

/// A returned record id (the create/update `RETURN AFTER` projection).
#[derive(surrealdb_types::SurrealValue)]
struct IdRow {
    id: RecordId,
}

/// One stored payload row, read back from `bandit_log`. Holds the opaque
/// `surrealdb_types::Value` (the SDK's `take` needs `SurrealValue`, not serde).
#[derive(surrealdb_types::SurrealValue)]
struct BanditPayloadRow {
    payload: surrealdb_types::Value,
}

/// Append one Layer-A RLVR bandit-log row (harness-rl Wave P2).
///
/// `payload` is the serialized `BanditRow` JSON (the `(x, a, p, r)` tuple) -- this
/// crate stays decoupled from `kavach-patterns`, so the typed row is serialized by
/// the caller and stored opaquely. Content-addressed by a BLAKE3 digest of the
/// payload so an identical replayed decision dedups to one row (append-only,
/// idempotent). Single-writer invariant: only the daemon reaches this.
///
/// # Errors
/// Returns an error if the `bandit_log` create fails.
pub async fn append_bandit_row(db: &Surreal<Db>, payload: &str) -> Result<RecordId> {
    let row_key = content_key(payload);
    let payload_value: serde_json::Value = serde_json::from_str(payload)
        .unwrap_or_else(|_| serde_json::Value::String(payload.to_owned()));
    // SurrealDB 3.0 renamed the SurrealQL builtin type::thing() -> type::record()
    // (parse-errors at runtime otherwise; same migration as parts.rs / projects.rs).
    let query = "CREATE type::record('bandit_log', $key) SET \
                 payload = $payload, created_at = time::now() RETURN AFTER";
    let mut response = db
        .query(query)
        .bind(("key", row_key))
        .bind(("payload", payload_value))
        .await?;
    let result: Option<IdRow> = response.take(0)?;
    match result {
        Some(e) => Ok(e.id),
        None => Err(crate::error::Error::RecordNotFound("bandit_log create".into())),
    }
}

/// Read back the stored bandit-log payloads, newest first, capped at `limit`.
///
/// Each returned string is the serialized `BanditRow` JSON the OPE layer
/// (kavach-ope) deserializes into a `LoggedSample`. The store keeps the payload
/// as an opaque `SurrealDB` value, so this bridges it to a JSON string at the
/// boundary (same `Value` -> `serde_json` hop as `db_harness/read.rs`).
///
/// # Errors
/// Returns an error if the `SELECT` fails or a stored payload is malformed.
pub async fn list_bandit_rows(db: &Surreal<Db>, limit: u32) -> Result<Vec<String>> {
    let query = "SELECT payload, created_at FROM bandit_log ORDER BY created_at DESC LIMIT $limit";
    let mut response = db.query(query).bind(("limit", i64::from(limit))).await?;
    let rows: Vec<BanditPayloadRow> = response.take(0)?;
    rows.into_iter().map(payload_to_json).collect()
}

/// List logged decisions whose reward has NOT been back-filled yet (P3a input).
///
/// A row is un-rewarded when its stored `payload.reward` is `null` — the emitter
/// logs `(x, a, p)` and defers `r` until the 3-witness verify resolves. The
/// back-fill writer fetches these as their `BanditRow` JSON, joins each to its
/// later verify outcome, and calls [`update_bandit_reward`] (which re-derives
/// the content-addressed key from the same payload) with the
/// `kavach_ope::label`-derived reward string.
///
/// # Errors
/// Returns an error if the `SELECT` fails or a stored payload is malformed.
pub async fn list_unrewarded_bandit_rows(db: &Surreal<Db>, limit: u32) -> Result<Vec<String>> {
    // Filter "un-rewarded" in Rust, not SurrealQL: a pending row stores `reward`
    // as JSON null, and SurrealDB 3.0's NULL-vs-NONE distinction makes a server
    // `WHERE payload.reward IS NONE` brittle (NULL is present-but-empty, not
    // NONE). Reading the reward back as plain JSON and testing `is_null()` is
    // version-proof. `limit` is applied AFTER the filter so it bounds candidates.
    let rows = select_all_payloads(db).await?;
    rows.into_iter()
        // A malformed payload (Err) is kept so it surfaces, not silently hidden.
        .filter(|res| res.as_ref().map_or(true, |s| reward_is_absent(s)))
        .take(limit as usize)
        .collect()
}

/// List a SINGLE session's un-rewarded decisions — the P3a JOIN input.
///
/// This is the join the reward back-fill needs: the stop gate knows the
/// `session_id` it is closing and that session's 3-witness verify outcome, so it
/// grades exactly the rows logged under this session. Filtering by `session_id`
/// here (the join key already on every `BanditRow`) is what closes the loop —
/// no separate per-decision verify-outcome emit is required, because the
/// session-level outcome applies to every decision logged within it.
///
/// Both the reward-absent and `session_id` filters run in Rust, not `SurrealQL`.
/// `SurrealDB` 3.x only turns a `WHERE` into an index/record-id scan when the
/// predicate is on the record id; `session_id` is a nested field on the opaque
/// payload, so a server-side `WHERE` would still table-scan (no index win) while
/// re-introducing the NULL-vs-NONE chaining brittleness the reward filter already
/// avoids. `limit` bounds the result AFTER both filters.
///
/// # Errors
/// Returns an error if the `SELECT` fails or a stored payload is malformed.
pub async fn list_unrewarded_bandit_rows_for_session(
    db: &Surreal<Db>,
    session_id: &str,
    limit: u32,
) -> Result<Vec<String>> {
    let rows = select_all_payloads(db).await?;
    rows.into_iter()
        // A malformed payload (Err) is kept so it surfaces, never silently hidden.
        .filter(|res| {
            res.as_ref().map_or(true, |s| reward_is_absent(s) && row_is_for_session(s, session_id))
        })
        .take(limit as usize)
        .collect()
}

/// Back-fill the realized reward on one logged decision (P3a write).
///
/// Re-derives the content-addressed key from the ORIGINAL un-rewarded `payload`
/// (BLAKE3, exactly as `append_bandit_row` did), so the caller passes the same
/// JSON it read from [`list_unrewarded_bandit_rows`] — no fragile record-id
/// parsing. `reward` is the `kavach_ope::label`-derived enum string
/// (`verified_clean` / `needed_ask` / `false_decision`) the OPE estimators read.
/// Idempotent: a re-run with the same payload + reward writes the same scalar.
///
/// # Errors
/// Returns an error if the `UPDATE` fails or no row matches the derived key.
pub async fn update_bandit_reward(db: &Surreal<Db>, payload: &str, reward: &str) -> Result<()> {
    let row_key = content_key(payload);
    let query = "UPDATE type::record('bandit_log', $key) SET payload.reward = $reward RETURN AFTER";
    let mut response = db
        .query(query)
        .bind(("key", row_key.clone()))
        .bind(("reward", reward.to_owned()))
        .await?;
    let updated: Vec<BanditPayloadRow> = response.take(0)?;
    if updated.is_empty() {
        return Err(crate::error::Error::RecordNotFound(format!(
            "bandit_log reward back-fill: no row for key {row_key}"
        )));
    }
    Ok(())
}

/// The content-addressed record key for a payload: the first 32 hex of its
/// BLAKE3 digest. The single definition both the append and the back-fill share,
/// so the key derivation can never drift between write and update.
fn content_key(payload: &str) -> String {
    let digest = blake3::hash(payload.as_bytes()).to_hex();
    digest.as_str().get(..32).unwrap_or(digest.as_str()).to_owned()
}

/// Fetch every stored payload as JSON, newest first (unbounded — callers filter
/// then `take`). The shared read both un-rewarded listers build on.
async fn select_all_payloads(db: &Surreal<Db>) -> Result<Vec<Result<String>>> {
    let query = "SELECT payload, created_at FROM bandit_log ORDER BY created_at DESC";
    let mut response = db.query(query).await?;
    let rows: Vec<BanditPayloadRow> = response.take(0)?;
    Ok(rows.into_iter().map(payload_to_json).collect())
}

/// Bridge one opaque stored payload to its plain-JSON string.
///
/// `into_json_value` emits PLAIN JSON; `serde_json::to_value` would emit
/// `SurrealDB`'s tagged enum form (`{"Object":{"action":{"String":..}}}`) that the
/// OPE estimators cannot parse.
fn payload_to_json(r: BanditPayloadRow) -> Result<String> {
    let json = r.payload.into_json_value();
    serde_json::to_string(&json).map_err(crate::error::Error::Json)
}

/// True when the serialized `BanditRow` JSON carries no realized reward yet —
/// `reward` is absent or JSON null. A parse failure counts as "still pending" so
/// a malformed row is surfaced to the caller, never silently graded.
fn reward_is_absent(payload: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(payload)
        .ok()
        .and_then(|v| v.get("reward").cloned())
        .is_none_or(|r| r.is_null())
}

/// True when the serialized `BanditRow` JSON was logged under `session_id`. A
/// parse failure counts as a match so a malformed row still surfaces to the
/// caller rather than being silently dropped from the grading set.
fn row_is_for_session(payload: &str, session_id: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(payload).ok().is_none_or(|v| {
        v.get("session_id").and_then(serde_json::Value::as_str) == Some(session_id)
    })
}

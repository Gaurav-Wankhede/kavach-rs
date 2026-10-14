// split: intentional — Layer-A RLVR bandit_log store (harness-rl); kept out of
// the generic write.rs so the self-improving-gate subsystem owns its own file.
// sql-safe: queries use static literals + params, no user input concat.
//! `bandit_log` persistence — the RLVR reward substrate (harness-rl Waves P2/P3a).
//!
//! Append-only, content-addressed decision rows `(context x, action a,
//! propensity p, reward r)`; `r` is logged `None` at decision time and
//! back-filled DOWNSTREAM once the 3-witness verify resolves. This crate stays
//! decoupled from `kavach-patterns`: the typed `BanditRow` is serialized by the
//! caller and stored opaquely as JSON.
use crate::error::Result;
use crate::db::DbStore;
#[cfg(test)]
#[path = "bandit_test.rs"]
mod tests;
/// Append one Layer-A RLVR bandit-log row (harness-rl Wave P2).
///
/// `payload` is the serialized `BanditRow` JSON (the `(x, a, p, r)` tuple) -- this
/// crate stays decoupled from `kavach-patterns`, so the typed row is serialized by
/// the caller and stored opaquely. Content-addressed by a BLAKE3 digest of the
/// payload so an identical replayed decision dedups to one row (append-only,
/// idempotent). Single-writer invariant: only the RPC server reaches this.
///
/// # Errors
/// Returns an error if the `bandit_log` insert fails.
pub async fn append_bandit_row(db: &DbStore, payload: &str) -> Result<String> {
    let row_key = content_key(payload);
    let now = crate::db::now_rfc3339();
    db.exec_params(
        "INSERT INTO bandit_log (key, payload, created_at) VALUES ($k, $p, $now) \
         ON CONFLICT(key) DO NOTHING",
        libsql::params![row_key.clone(), payload.to_owned(), now],
    )
    .await?;
    Ok(row_key)
}
/// Read back the stored bandit-log payloads, newest first, capped at `limit`.
///
/// Each returned string is the serialized `BanditRow` JSON the OPE layer
/// (kavach-ope) deserializes into a `LoggedSample`.
///
/// # Errors
/// Returns an error if the `SELECT` fails or a stored payload is malformed.
pub async fn list_bandit_rows(db: &DbStore, limit: u32) -> Result<Vec<String>> {
    let mut rows = db
        .exec_params(
            "SELECT payload FROM bandit_log ORDER BY created_at DESC LIMIT $limit",
            libsql::params![i64::from(limit)],
        )
        .await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push(row.get::<String>(0)?);
    }
    Ok(out)
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
pub async fn list_unrewarded_bandit_rows(db: &DbStore, limit: u32) -> Result<Vec<String>> {
    // Filter "un-rewarded" in Rust, not SQL: a pending row stores `reward` as
    // JSON null. Reading the payload back as plain JSON and testing `is_null()`
    // is version-proof. `limit` is applied AFTER the filter so it bounds candidates.
    let rows = select_all_payloads(db).await?;
    rows.into_iter()
        // A malformed payload (Err) is kept so it surfaces, not silently hidden.
        .filter(|res| res.as_ref().map_or(true, |s| reward_is_absent(s)))
        .take(limit as usize)
        .collect::<std::result::Result<Vec<_>, _>>()
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
/// Both the reward-absent and `session_id` filters run in Rust, not SQL.
/// `limit` bounds the result AFTER both filters.
///
/// # Errors
/// Returns an error if the `SELECT` fails or a stored payload is malformed.
pub async fn list_unrewarded_bandit_rows_for_session(
    db: &DbStore,
    session_id: &str,
    limit: u32,
) -> Result<Vec<String>> {
    let rows = select_all_payloads(db).await?;
    rows.into_iter()
        // A malformed payload (Err) is kept so it surfaces, never silently hidden.
        // Parse ONCE per row and test both predicates on the same Value (the
        // per-session list is the hottest reader — re-parsing each row twice was
        // pure waste).
        .filter(|res| {
            res.as_ref()
                .map_or(true, |s| pending_for_session(s, session_id))
        })
        .take(limit as usize)
        .collect::<std::result::Result<Vec<_>, _>>()
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
pub async fn update_bandit_reward(db: &DbStore, payload: &str, reward: &str) -> Result<()> {
    let row_key = content_key(payload);
    let mut payload_value: serde_json::Value =
        serde_json::from_str(payload).map_err(crate::error::Error::Json)?;
    let payload_obj = payload_value.as_object_mut().ok_or_else(|| {
        crate::error::Error::Validation("bandit payload must be a JSON object".into())
    })?;
    payload_obj.insert(
        "reward".to_owned(),
        serde_json::Value::String(reward.to_owned()),
    );
    let updated_payload =
        serde_json::to_string(&payload_value).map_err(crate::error::Error::Json)?;
    let now = crate::db::now_rfc3339();
    let mut resp = db
        .exec_params(
            "UPDATE bandit_log SET payload = $p, created_at = $now WHERE key = $k \
             RETURNING key",
            libsql::params![updated_payload, now, row_key.clone()],
        )
        .await?;
    if resp.next().await?.is_none() {
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
    digest
        .as_str()
        .get(..32)
        .unwrap_or(digest.as_str())
        .to_owned()
}
/// Fetch every stored payload as JSON, newest first (unbounded — callers filter
/// then `take`). The shared read both un-rewarded listers build on.
async fn select_all_payloads(db: &DbStore) -> Result<Vec<Result<String>>> {
    let mut rows = db
        .exec_params(
            "SELECT payload FROM bandit_log ORDER BY created_at DESC",
            (),
        )
        .await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push(Ok(row.get::<String>(0)?));
    }
    Ok(out)
}
/// True when the serialized `BanditRow` JSON carries no realized reward yet —
/// `reward` is absent or JSON null. A parse failure counts as "still pending" so
/// a malformed row is surfaced to the caller, never silently graded.
fn reward_is_absent(payload: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(payload)
        .ok()
        .is_none_or(|v| reward_absent_in(&v))
}
/// True when this row is un-rewarded AND logged under `session_id` — the
/// per-session JOIN predicate. Parses the payload ONCE and tests both fields on
/// the same `Value`; a parse failure surfaces the row (both predicates default
/// to "kept") rather than silently dropping it from the grading set.
fn pending_for_session(payload: &str, session_id: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(payload)
        .ok()
        .is_none_or(|v| reward_absent_in(&v) && session_matches_in(&v, session_id))
}
/// `reward` field is absent or JSON null on an already-parsed row.
fn reward_absent_in(v: &serde_json::Value) -> bool {
    v.get("reward").is_none_or(serde_json::Value::is_null)
}
/// `session_id` field equals `session_id` on an already-parsed row.
fn session_matches_in(v: &serde_json::Value, session_id: &str) -> bool {
    v.get("session_id").and_then(serde_json::Value::as_str) == Some(session_id)
}

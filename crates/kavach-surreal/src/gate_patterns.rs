// split: intentional - cohesive gate_pattern store (struct + 3 async DB ops + pure helpers)
// SurrealDB-backed gate_pattern store. Mirrors kavach-db::gate_patterns API.
// Tokenization, bloom filter, and TF-IDF scoring are pure-fn ports.
// SDK ref: surrealdb 3.1.4 — CREATE ... RETURN id + take(0) into typed struct.
// sql-safe: explicit column list; bound params only; no string concat.
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::{RecordId, SurrealValue};
const TIER_RESEARCH: &str = "research";
const TIER_AUTONOMOUS: &str = "autonomous";
const PROMOTION_THRESHOLD: i64 = 50;
const BLOOM_BITS: usize = 512;
// Bit-to-byte conversion via shift (>>3 = /8 for power-of-2). Exact for any
// `BLOOM_BITS` that is a multiple of 8; clippy::integer_division would
// flag the `/` form even though both sides are compile-time constants.
const BLOOM_BYTE_LEN: usize = BLOOM_BITS >> 3;
const FNV_SEED_1: u32 = 0x811c_9dc5;
const FNV_SEED_2: u32 = 0xdead_beef;
const FNV_PRIME: u32 = 0x0100_0193;
const MAX_TOKENS: usize = 20;
const MIN_TOKEN_LEN: usize = 3;
const SCAN_LIMIT: i64 = 200;
const MIN_SIM: f64 = 0.35;
const COLS: &str = "id, project, tool_name, gate_name, error_tokens, fix_strategy, \
                    imperative_rewrite, dsa_rationale, occurrence_count, bloom_bytes, tier, \
                    time::unix(updated_at) AS updated_unix";
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[non_exhaustive]
pub struct GatePattern {
    pub id: Option<RecordId>,
    pub project: RecordId,
    pub tool_name: String,
    pub gate_name: String,
    pub error_tokens: String,
    pub fix_strategy: String,
    pub imperative_rewrite: String,
    pub dsa_rationale: String,
    pub occurrence_count: i64,
    #[serde(default)]
    pub bloom_bytes: Option<Vec<u8>>,
    pub tier: String,
    /// Unix epoch of last touch (`time::unix(updated_at)`), driving the `k_pri`
    /// recency axis when injection-ranking. `None` on legacy rows.
    #[serde(default)]
    pub updated_unix: Option<i64>,
}
#[derive(Debug)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate struct-literal DTO (kavach-rpc); non_exhaustive => E0639"
)]
pub struct UpsertParams<'a> {
    pub project: RecordId,
    pub error_tokens: &'a str,
    pub fix_strategy: &'a str,
    pub imperative_rewrite: &'a str,
    pub dsa_rationale: &'a str,
    pub tool_name: &'a str,
    pub gate_name: &'a str,
}
#[must_use]
pub fn tokenize(error: &str) -> String {
    let mut tokens: Vec<String> = error
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= MIN_TOKEN_LEN)
        .map(str::to_lowercase)
        .collect();
    tokens.sort_unstable();
    tokens.dedup();
    tokens.truncate(MAX_TOKENS);
    tokens.join(" ")
}
#[must_use]
pub fn bloom_from_tokens(tokens: &str) -> Vec<u8> {
    let mut bits = vec![0u8; BLOOM_BYTE_LEN];
    for token in tokens.split_whitespace() {
        let h1 = fnv1a(token, FNV_SEED_1);
        let h2 = fnv1a(token, FNV_SEED_2);
        set_bit(&mut bits, (h1 as usize) % BLOOM_BITS);
        set_bit(&mut bits, (h2 as usize) % BLOOM_BITS);
    }
    bits
}
#[must_use]
pub fn bloom_might_match(bloom: &[u8], query_tokens: &str) -> bool {
    if bloom.len() < BLOOM_BYTE_LEN {
        return true;
    }
    for token in query_tokens.split_whitespace() {
        let h1 = fnv1a(token, FNV_SEED_1);
        let h2 = fnv1a(token, FNV_SEED_2);
        if !test_bit(bloom, (h1 as usize) % BLOOM_BITS)
            || !test_bit(bloom, (h2 as usize) % BLOOM_BITS)
        {
            return false;
        }
    }
    true
}
fn set_bit(bytes: &mut [u8], pos: usize) {
    // `pos >> 3` ≡ `pos / 8`, `pos & 7` ≡ `pos % 8` for bit-packed bytes;
    // avoids clippy::integer_division on a known-exact byte-index calc.
    if let Some(byte) = bytes.get_mut(pos >> 3) {
        *byte |= 1u8 << (pos & 7);
    }
}
fn test_bit(bytes: &[u8], pos: usize) -> bool {
    bytes
        .get(pos >> 3)
        .is_some_and(|b| b & (1u8 << (pos & 7)) != 0)
}
fn fnv1a(s: &str, seed: u32) -> u32 {
    let mut h = seed;
    for b in s.bytes() {
        h ^= u32::from(b);
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}
#[derive(surrealdb_types::SurrealValue)]
struct ExistingRow {
    id: RecordId,
    occurrence_count: i64,
}
#[derive(surrealdb_types::SurrealValue)]
struct IdRow {
    id: RecordId,
}
/// Upsert a `gate_pattern` row. Increments `occurrence_count` for an existing
/// (project, `error_tokens`) pair or creates a new row in `research` tier.
///
/// # Errors
/// Returns `Error::Surreal` when the SELECT/UPDATE/CREATE query fails, and
/// `Error::Migration` when the CREATE response yields no id row.
pub async fn upsert(db: &Surreal<Db>, p: &UpsertParams<'_>) -> Result<RecordId> {
    let tokens = tokenize(p.error_tokens);
    let find = "SELECT id, occurrence_count FROM gate_pattern \
                WHERE project = $project AND error_tokens = $tokens LIMIT 1";
    let mut response = db
        .query(find)
        .bind(("project", p.project.clone()))
        .bind(("tokens", tokens.clone()))
        .await?;
    let existing: Option<ExistingRow> = response.take(0)?;
    if let Some(row) = existing {
        let new_count = row.occurrence_count.saturating_add(1);
        let (tier, bloom): (&str, Option<Vec<u8>>) = if new_count >= PROMOTION_THRESHOLD {
            (TIER_AUTONOMOUS, Some(bloom_from_tokens(&tokens)))
        } else {
            (TIER_RESEARCH, None)
        };
        let upd = "UPDATE $id SET \
                   occurrence_count = $count, fix_strategy = $fix, \
                   imperative_rewrite = $rewrite, dsa_rationale = $dsa, \
                   tier = $tier, bloom_bytes = $bloom, updated_at = time::now()";
        db.query(upd)
            .bind(("id", row.id.clone()))
            .bind(("count", new_count))
            .bind(("fix", p.fix_strategy.to_owned()))
            .bind(("rewrite", p.imperative_rewrite.to_owned()))
            .bind(("dsa", p.dsa_rationale.to_owned()))
            .bind(("tier", tier.to_owned()))
            .bind(("bloom", bloom))
            .await?;
        Ok(row.id)
    } else {
        let ins = "CREATE gate_pattern SET \
                   project = $project, tool_name = $tool, gate_name = $gate, \
                   error_tokens = $tokens, fix_strategy = $fix, \
                   imperative_rewrite = $rewrite, dsa_rationale = $dsa, \
                   occurrence_count = 1, tier = 'research' \
                   RETURN id";
        let mut resp = db
            .query(ins)
            .bind(("project", p.project.clone()))
            .bind(("tool", p.tool_name.to_owned()))
            .bind(("gate", p.gate_name.to_owned()))
            .bind(("tokens", tokens))
            .bind(("fix", p.fix_strategy.to_owned()))
            .bind(("rewrite", p.imperative_rewrite.to_owned()))
            .bind(("dsa", p.dsa_rationale.to_owned()))
            .await?;
        let row: Option<IdRow> = resp.take(0)?;
        row.map(|ir| ir.id)
            .ok_or_else(|| Error::Migration("gate_pattern create returned no id".into()))
    }
}
/// Find the best-matching autonomous-tier `gate_pattern` for `error` via
/// TF-IDF scored against the candidate set scoped to `project`.
///
/// # Errors
/// Returns `Error::Surreal` when the candidate SELECT fails to execute or
/// deserialize.
pub async fn find_autonomous(
    db: &Surreal<Db>,
    project: &RecordId,
    error: &str,
    tool_name: &str,
) -> Result<Option<GatePattern>> {
    let query_tokens = tokenize(error);
    let query_vec: Vec<&str> = query_tokens.split_whitespace().collect();
    if query_vec.is_empty() {
        return Ok(None);
    }
    let q = format!(
        "SELECT {COLS} FROM gate_pattern \
         WHERE project = $project AND tier = 'autonomous' \
           AND (tool_name = '' OR tool_name = $tool) \
         LIMIT $limit"
    );
    let mut response = db
        .query(q)
        .bind(("project", project.clone()))
        .bind(("tool", tool_name.to_owned()))
        .bind(("limit", SCAN_LIMIT))
        .await?;
    // Missing `gate_pattern` table (no pattern ever recorded) is the empty case,
    // not a failure — same as `list_hot`.
    let candidates: Vec<GatePattern> = match response.take(0) {
        Ok(c) => c,
        Err(e) if crate::error::is_missing_table_error(&e) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    if candidates.is_empty() {
        return Ok(None);
    }
    Ok(tfidf_best_match(candidates, &query_vec, &query_tokens))
}
#[expect(
    clippy::float_arithmetic,
    clippy::cast_precision_loss,
    reason = "TF-IDF cosine over candidate-set sizes <= SCAN_LIMIT (200); f64 \
              precision sufficient for similarity ranking, not absolute scoring"
)]
fn tfidf_best_match(
    candidates: Vec<GatePattern>,
    query_vec: &[&str],
    query_tokens: &str,
) -> Option<GatePattern> {
    let n = candidates.len() as f64;
    let mut df = std::collections::HashMap::new();
    for token in query_vec {
        let count = candidates
            .iter()
            .filter(|p| p.error_tokens.split_whitespace().any(|t| t == *token))
            .count() as f64;
        if count > 0.0 {
            df.insert(*token, count);
        }
    }
    let idf = |token: &str| -> f64 { df.get(token).map_or(0.0, |&d| (n / d).ln()) };
    let query_norm: f64 = query_vec
        .iter()
        .map(|t| {
            let w = idf(t);
            w * w
        })
        .sum::<f64>()
        .sqrt();
    if query_norm == 0.0 {
        return None;
    }
    candidates
        .into_iter()
        .filter(|c| {
            c.bloom_bytes
                .as_ref()
                .is_none_or(|b| bloom_might_match(b, query_tokens))
        })
        .filter_map(|candidate| {
            let pat_str = candidate.error_tokens.clone();
            let pat_tokens: Vec<&str> = pat_str.split_whitespace().collect();
            let dot: f64 = query_vec
                .iter()
                .filter(|t| pat_tokens.iter().any(|p| p == *t))
                .map(|t| {
                    let w = idf(t);
                    w * w
                })
                .sum();
            let pat_sq: f64 = pat_tokens
                .iter()
                .filter(|t| query_vec.iter().any(|q| q == *t))
                .map(|t| {
                    let w = idf(t);
                    w * w
                })
                .sum();
            let pat_norm = pat_sq.sqrt();
            if pat_norm == 0.0 {
                return None;
            }
            let sim = dot / (query_norm * pat_norm);
            (sim >= MIN_SIM).then_some((sim, candidate))
        })
        .reduce(|(best_sim, best_pat), (sim, pat)| {
            if sim > best_sim {
                (sim, pat)
            } else {
                (best_sim, best_pat)
            }
        })
        .map(|(_, pat)| pat)
}
/// List the hottest (highest `occurrence_count`) `gate_pattern` rows for a project.
///
/// # Errors
/// Propagates `Error::Surreal` from the SELECT.
pub async fn list_hot(
    db: &Surreal<Db>,
    project: &RecordId,
    limit: usize,
) -> Result<Vec<GatePattern>> {
    let q = format!(
        "SELECT {COLS} FROM gate_pattern \
         WHERE project = $project AND tier = 'autonomous' \
         ORDER BY occurrence_count DESC \
         LIMIT $limit"
    );
    let limit_i64 = i64::try_from(limit).unwrap_or(i64::MAX);
    let mut response = db
        .query(q)
        .bind(("project", project.clone()))
        .bind(("limit", limit_i64))
        .await?;
    // A daemon DB that never recorded an autonomous pattern has no `gate_pattern`
    // table yet, so SELECT raises "table does not exist" — the empty case (zero
    // hot patterns), not a failure. Mirrors `top_deployed_policies`.
    match response.take(0) {
        Ok(rows) => Ok(rows),
        Err(e) if crate::error::is_missing_table_error(&e) => Ok(Vec::new()),
        Err(e) => Err(e.into()),
    }
}
#[cfg(test)]
#[path = "gate_patterns_test.rs"]
mod tests;

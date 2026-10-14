use crate::db::DbStore;
// split: intentional - cohesive gate_pattern store (struct + 3 async DB ops + pure helpers)
// Database-backed gate_pattern store. Mirrors kavach-db::gate_patterns API.
// Tokenization, bloom filter, and TF-IDF scoring are pure-fn ports.
// sql-safe: explicit column list; bound params only; no string concat.
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
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
// Column order for gate_pattern rows: id, project, tool_name, gate_name,
// error_tokens, fix_strategy, imperative_rewrite, dsa_rationale,
// occurrence_count, bloom_bytes, tier, updated_at
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GatePattern {
    pub id: Option<String>,
    pub project: String,
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
    /// Unix epoch of last touch, driving the `k_pri` recency axis when
    /// injection-ranking. `None` on legacy rows.
    #[serde(default)]
    pub updated_unix: Option<i64>,
}
#[derive(Debug)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate struct-literal DTO (kavach-rpc); non_exhaustive => E0639"
)]
pub struct UpsertParams<'a> {
    pub project: String,
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
struct ExistingRow {
    id: String,
    occurrence_count: i64,
}
/// Map a `gate_pattern` row (column order per `COLS`) into a [`GatePattern`].
fn pattern_from_row(r: &libsql::Row) -> Result<GatePattern> {
    let updated_at: Option<String> = r.get(11)?;
    Ok(GatePattern {
        id: Some(r.get::<String>(0)?),
        project: r.get::<String>(1)?,
        tool_name: r.get::<String>(2)?,
        gate_name: r.get::<String>(3)?,
        error_tokens: r.get::<String>(4)?,
        fix_strategy: r.get::<String>(5)?,
        imperative_rewrite: r.get::<String>(6)?,
        dsa_rationale: r.get::<String>(7)?,
        occurrence_count: r.get::<i64>(8)?,
        bloom_bytes: r.get::<Option<Vec<u8>>>(9)?,
        tier: r.get::<String>(10)?,
        updated_unix: updated_at
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|d| d.timestamp()),
    })
}
/// True when the error is libsql's "no such table" — the empty-store case for
/// tables created lazily on first write. Mirrors the old `is_missing_table_error`.
fn is_missing_table(e: &libsql::Error) -> bool {
    e.to_string().contains("no such table")
}
/// Upsert a `gate_pattern` row. Increments `occurrence_count` for an existing
/// (project, `error_tokens`) pair or creates a new row in `research` tier.
///
/// # Errors
/// Returns `Error::Database` when the SELECT/UPDATE/CREATE query fails, and
/// `Error::Migration` when the CREATE response yields no id row.
pub async fn upsert(db: &DbStore, p: &UpsertParams<'_>) -> Result<String> {
    let tokens = tokenize(p.error_tokens);
    let mut response = db
        .exec_params(
            "SELECT id, occurrence_count FROM gate_pattern \
             WHERE project = $project AND error_tokens = $tokens LIMIT 1",
            libsql::params![p.project.clone(), tokens.clone()],
        )
        .await?;
    let existing: Option<ExistingRow> = match response.next().await? {
        Some(r) => Some(ExistingRow {
            id: r.get::<String>(0)?,
            occurrence_count: r.get::<i64>(1)?,
        }),
        None => None,
    };
    if let Some(row) = existing {
        let new_count = row.occurrence_count.saturating_add(1);
        let (tier, bloom): (&str, Option<Vec<u8>>) = if new_count >= PROMOTION_THRESHOLD {
            (TIER_AUTONOMOUS, Some(bloom_from_tokens(&tokens)))
        } else {
            (TIER_RESEARCH, None)
        };
        let now = crate::db::now_rfc3339();
        db.exec_params(
            "UPDATE gate_pattern SET \
               occurrence_count = $count, fix_strategy = $fix, \
               imperative_rewrite = $rewrite, dsa_rationale = $dsa, \
               tier = $tier, bloom_bytes = $bloom, updated_at = $now \
             WHERE id = $id",
            libsql::params![
                new_count,
                p.fix_strategy.to_owned(),
                p.imperative_rewrite.to_owned(),
                p.dsa_rationale.to_owned(),
                tier.to_owned(),
                bloom,
                now,
                row.id.clone()
            ],
        )
        .await?;
        Ok(row.id)
    } else {
        let id = crate::decisions::hash_keyed("gate_pattern", &p.project, &tokens, p.tool_name);
        let now = crate::db::now_rfc3339();
        let mut resp = db
            .exec_params(
                "INSERT INTO gate_pattern \
                     (id, project, tool_name, gate_name, error_tokens, fix_strategy, \
                      imperative_rewrite, dsa_rationale, occurrence_count, tier, updated_at) \
                 VALUES ($id, $project, $tool, $gate, $tokens, $fix, $rewrite, $dsa, 1, 'research', $now) \
                 ON CONFLICT(id) DO NOTHING \
                 RETURNING id",
                libsql::params![
                    id.clone(),
                    p.project.clone(),
                    p.tool_name.to_owned(),
                    p.gate_name.to_owned(),
                    tokens,
                    p.fix_strategy.to_owned(),
                    p.imperative_rewrite.to_owned(),
                    p.dsa_rationale.to_owned(),
                    now
                ],
            )
            .await?;
        match resp.next().await? {
            Some(_) => Ok(id),
            None => Err(Error::Migration(
                "gate_pattern create returned no id".into(),
            )),
        }
    }
}
/// Find the best-matching autonomous-tier `gate_pattern` for `error` via
/// TF-IDF scored against the candidate set scoped to `project`.
///
/// # Errors
/// Returns `Error::Database` when the candidate SELECT fails to execute or
/// deserialize.
pub async fn find_autonomous(
    db: &DbStore,
    project: &str,
    error: &str,
    tool_name: &str,
) -> Result<Option<GatePattern>> {
    let query_tokens = tokenize(error);
    let query_vec: Vec<&str> = query_tokens.split_whitespace().collect();
    if query_vec.is_empty() {
        return Ok(None);
    }
    let mut response = db
        .exec_params(
            "SELECT id, project, tool_name, gate_name, error_tokens, fix_strategy, \
                    imperative_rewrite, dsa_rationale, occurrence_count, bloom_bytes, tier, \
                    updated_at \
             FROM gate_pattern \
             WHERE project = $project AND tier = 'autonomous' \
               AND (tool_name = '' OR tool_name = $tool) \
             LIMIT $limit",
            libsql::params![project.to_owned(), tool_name.to_owned(), SCAN_LIMIT],
        )
        .await?;
    // Missing `gate_pattern` table (no pattern ever recorded) is the empty case,
    // not a failure — same as `list_hot`.
    let mut candidates: Vec<GatePattern> = Vec::new();
    loop {
        match response.next().await {
            Ok(Some(r)) => candidates.push(pattern_from_row(&r)?),
            Ok(None) => break,
            Err(e) if is_missing_table(&e) => return Ok(None),
            Err(e) => return Err(e.into()),
        }
    }
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
/// Propagates `Error::Database` from the SELECT.
pub async fn list_hot(db: &DbStore, project: &str, limit: usize) -> Result<Vec<GatePattern>> {
    let limit_i64 = i64::try_from(limit).unwrap_or(i64::MAX);
    let mut response = db
        .exec_params(
            "SELECT id, project, tool_name, gate_name, error_tokens, fix_strategy, \
                    imperative_rewrite, dsa_rationale, occurrence_count, bloom_bytes, tier, \
                    updated_at \
             FROM gate_pattern \
             WHERE project = $project AND tier = 'autonomous' \
             ORDER BY occurrence_count DESC \
             LIMIT $limit",
            libsql::params![project.to_owned(), limit_i64],
        )
        .await?;
    // A daemon DB that never recorded an autonomous pattern has no `gate_pattern`
    // table yet, so SELECT raises "no such table" — the empty case (zero
    // hot patterns), not a failure. Mirrors `top_deployed_policies`.
    let mut out = Vec::new();
    loop {
        match response.next().await {
            Ok(Some(r)) => out.push(pattern_from_row(&r)?),
            Ok(None) => break,
            Err(e) if is_missing_table(&e) => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(out)
}
#[cfg(test)]
#[path = "gate_patterns_test.rs"]
mod tests;

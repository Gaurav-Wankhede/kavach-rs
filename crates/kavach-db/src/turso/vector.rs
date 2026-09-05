//! Vector embedding + hybrid recall — Phase 5.
//!
//! Embedding storage is `F32_BLOB(384)` (little-endian f32 arrays).
//! Hybrid recall fuses FTS5-BM25, cosine ANN, and graph-edge expansion
//! via [`crate::rrf::rrf_fuse`] (k = 60, TREC canonical).

use std::future::Future;
use std::sync::OnceLock;

use super::{TursoError, TursoResult, TursoStore};
use crate::brain::BrainHit;
use crate::rrf::{RRF_K, rrf_fuse};
use crate::turso::search::{CORPUS_CATEGORIES, SearchStore};

/// Canonical embedding dimension.
///
/// Changing this after a DB has rows requires a full reindex.
pub const EMBEDDING_DIMS: usize = 384;

/// DDL owned by the vector domain. Executed best-effort by `connection::init_schema`.
pub const DDL: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_memories_embedding ON memories(libsql_vector_idx(embedding, 'metric=cosine'))",
];

/// Brute-force cosine similarity — `[-1, 1]`; higher ⇒ more similar.
#[must_use]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    let mut dot = 0.0_f64;
    let mut na = 0.0_f64;
    let mut nb = 0.0_f64;
    for i in 0..a.len() {
        let av = f64::from(*a.get(i).unwrap_or(&0.0_f32));
        let bv = f64::from(*b.get(i).unwrap_or(&0.0_f32));
        dot = av.mul_add(bv, dot);
        na = av.mul_add(av, na);
        nb = bv.mul_add(bv, nb);
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    #[expect(clippy::float_arithmetic, reason = "cosine: bounded float math")]
    let denom = (na * nb).sqrt();
    if denom == 0.0 {
        0.0
    } else {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "cosine in [-1,1] fits f32 exactly"
        )]
        #[expect(clippy::float_arithmetic, reason = "cosine: bounded division")]
        {
            (dot / denom) as f32
        }
    }
}

/// Serialize a `&[f32]` into the little-endian blob expected by `F32_BLOB`.
#[must_use]
pub fn serialize_embedding(vec: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(vec.len().saturating_mul(4));
    for v in vec {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// Deserialize a `F32_BLOB` slice back to `Vec<f32>`.
///
/// Returns empty on malformed length — stored blobs are trusted, this guards
/// against truncated writes.
#[must_use]
pub fn deserialize_embedding(blob: &[u8]) -> Vec<f32> {
    if !blob.len().is_multiple_of(4) {
        return Vec::new();
    }
    #[expect(
        clippy::integer_division,
        reason = "blob.len() is proven multiple of 4 above; division is exact"
    )]
    let mut out = Vec::with_capacity(blob.len() / 4);
    for chunk in blob.chunks_exact(4) {
        let arr: [u8; 4] = [
            *chunk.first().unwrap_or(&0),
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
            *chunk.get(3).unwrap_or(&0),
        ];
        out.push(f32::from_le_bytes(arr));
    }
    out
}

/// Deterministic hash-seeded fallback embedding.
///
/// 384-D pseudo-random unit vector from `blake3(text)`. Used in CI offline
/// mode so tests are deterministic without the 120 MB ONNX model download.
#[must_use]
pub fn fallback_embed(text: &str) -> Vec<f32> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(text.as_bytes());
    let hash = hasher.finalize();
    let seed = *hash.as_bytes();
    let mut out = Vec::with_capacity(EMBEDDING_DIMS);
    let mut cur_seed = seed;
    for _ in 0..EMBEDDING_DIMS {
        let mut next_h = blake3::Hasher::new();
        next_h.update(&cur_seed);
        let digest = next_h.finalize();
        let digest_bytes = digest.as_bytes();
        let byte_arr: [u8; 4] = [
            *digest_bytes.first().unwrap_or(&0),
            *digest_bytes.get(1).unwrap_or(&0),
            *digest_bytes.get(2).unwrap_or(&0),
            *digest_bytes.get(3).unwrap_or(&0),
        ];
        let raw_u32 = u32::from_le_bytes(byte_arr);
        #[expect(
            clippy::float_arithmetic,
            reason = "fallback: bounded mapping to [-1,1]"
        )]
        #[expect(
            clippy::cast_possible_truncation,
            reason = "mul_add result in [-1,1]; truncation to f32 acceptable for fallback"
        )]
        let sample = f64::from(raw_u32).mul_add(2.0 / f64::from(u32::MAX), -1.0) as f32;
        out.push(sample);
        cur_seed = *digest.as_bytes();
    }
    // L2-normalize to unit sphere (cosine = dot product after normalization).
    let norm_sq: f64 = out
        .iter()
        .map(|v| {
            let vd = f64::from(*v);
            #[expect(clippy::float_arithmetic, reason = "norm: bounded squaring")]
            {
                vd * vd
            }
        })
        .sum();
    let norm_f64 = norm_sq.sqrt();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "norm >= 0.0; for unit vectors fits f32 cleanly"
    )]
    let norm_f32 = norm_f64 as f32;
    if norm_f32 > f32::EPSILON {
        for v in &mut out {
            #[expect(
                clippy::float_arithmetic,
                reason = "normalization; guarded by EPSILON check"
            )]
            {
                *v /= norm_f32;
            }
        }
    }
    out
}

/// Attempt real `fastembed` embedding for a batch of texts.
///
/// Returns `Ok(vecs)` when the model is cached; `Err` otherwise (caller falls
/// back to [`fallback_embed`]). The `OnceLock` avoids re-initialization cost.
///
/// # Errors
/// Returns [`TursoError`] when the model is unavailable or embed fails.
#[expect(
    clippy::significant_drop_tightening,
    reason = "model borrows from guard; drop(guard) after as_mut() fails borrow check"
)]
fn try_fastembed_batch(texts: &[String]) -> TursoResult<Vec<Vec<f32>>> {
    use fastembed::{EmbeddingModel, InitOptionsWithLength, TextEmbedding};
    // Mutex<Option<TextEmbedding>>: guard held across init AND embed
    // because TextEmbedding::embed takes &mut self. kavach:intentional
    static MODEL: OnceLock<std::sync::Mutex<Option<TextEmbedding>>> = OnceLock::new();
    let mutex = MODEL.get_or_init(|| std::sync::Mutex::new(None));
    let mut guard = match mutex.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    // Init once — offline / no-cache → leave None; caller uses fallback.
    if guard.is_none() {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("kavach")
            .join("embeddings");
        // SOURCE: docs.rs/fastembed/6.0.2 — InitOptionsWithLength::new() builder
        let opts = InitOptionsWithLength::new(EmbeddingModel::BGESmallENV15)
            .with_cache_dir(cache_dir)
            .with_show_download_progress(false);
        match TextEmbedding::try_new(opts) {
            Ok(model) => *guard = Some(model),
            Err(_) => {
                return Err(TursoError::Mapping("fastembed model not available".into()));
            }
        }
    }
    let model = guard
        .as_mut()
        .ok_or_else(|| TursoError::Mapping("fastembed unavailable".into()))?;
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    model
        .embed(refs, None)
        .map_err(|e| TursoError::Mapping(format!("fastembed embed: {e}")))
}

/// Embed `texts` — fastembed when cached, deterministic fallback otherwise.
///
/// Always returns `EMBEDDING_DIMS`-wide vectors, normalized for cosine.
///
/// # Errors
/// Returns [`TursoError`] only on internal logic failures (not model absence).
pub fn embed_texts(texts: &[String]) -> TursoResult<Vec<Vec<f32>>> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }
    Ok(try_fastembed_batch(texts)
        .unwrap_or_else(|_| texts.iter().map(|t| fallback_embed(t)).collect()))
}

/// Embed a single `title + content` concatenation (write-time helper).
///
/// # Errors
/// Propagates from [`embed_texts`].
pub fn embed_one(title: &str, content: &str) -> TursoResult<Vec<f32>> {
    let joined = if content.is_empty() {
        title.to_owned()
    } else {
        format!("{title}\n{content}")
    };
    let batch = embed_texts(&[joined])?;
    batch
        .into_iter()
        .next()
        .ok_or_else(|| TursoError::Mapping("embed_one empty".into()))
}

/// Execute `sql` but swallow the error when the libsql build lacks the vector
/// extension (error string contains `libsql_vector_idx` / `vector_top_k`).
pub(crate) async fn exec_vector_ddl_best_effort(store: &TursoStore, ddl: &str) -> TursoResult<()> {
    match store.conn.execute(ddl, ()).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("libsql_vector_idx")
                || msg.contains("vector_top_k")
                || msg.contains("no such function")
                || msg.contains("F32_BLOB")
            {
                Ok(())
            } else {
                Err(TursoError::from(e))
            }
        }
    }
}

/// Brute-force vector ranking — scans `memories.embedding` rows for `project`,
/// scores each against `query_vec` with cosine, sorts descending.
async fn brute_rank(
    store: &TursoStore,
    project: &str,
    query_vec: &[f32],
    limit: usize,
) -> TursoResult<Vec<String>> {
    let mut rows = store
        .exec_params(
            "SELECT entry_key, embedding FROM memories WHERE project = ?1 AND embedding IS NOT NULL",
            libsql::params![project],
        )
        .await?;
    let mut scored: Vec<(String, f32)> = Vec::new();
    while let Some(row) = rows.next().await.map_err(TursoError::from)? {
        let key = row.get::<String>(0).map_err(TursoError::from)?;
        let blob = row.get::<Vec<u8>>(1).map_err(TursoError::from)?;
        let vec = deserialize_embedding(&blob);
        if vec.len() != EMBEDDING_DIMS || vec.len() != query_vec.len() {
            continue;
        }
        let sim = cosine_similarity(query_vec, &vec);
        scored.push((key, sim));
    }
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);
    Ok(scored.into_iter().map(|(k, _)| k).collect())
}

/// Graph-expansion signal — 1-hop/2-hop closure over the `edges` memory DAG.
async fn graph_expand(
    store: &TursoStore,
    project: &str,
    seeds: &[String],
    depth: u32,
) -> TursoResult<Vec<String>> {
    if seeds.is_empty() {
        return Ok(Vec::new());
    }
    let mut out: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for seed in seeds {
        let sql = format!(
            "WITH RECURSIVE reach(dst_key, depth) AS ( \
               SELECT dst_key, 1 FROM edges WHERE src_project = ?1 AND src_key = ?2 \
               UNION \
               SELECT e.dst_key, reach.depth + 1 FROM edges e \
                 JOIN reach ON e.src_key = reach.dst_key \
                 WHERE e.src_project = ?1 AND reach.depth < {depth} \
             ) SELECT DISTINCT dst_key FROM reach ORDER BY dst_key"
        );
        let mut rows = store
            .exec_params(&sql, libsql::params![project, seed.clone()])
            .await?;
        while let Some(row) = rows.next().await.map_err(TursoError::from)? {
            let key = row.get::<String>(0).map_err(TursoError::from)?;
            if seen.insert(key.clone()) {
                out.push(key);
            }
        }
    }
    Ok(out)
}

/// Vector + hybrid operations on the embedded store.
pub trait VectorStore {
    /// Embed `query` and brute-rank `project`'s memories by cosine similarity.
    ///
    /// Empty query returns no hits.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn vector_rank(
        &self,
        project: &str,
        query: &str,
        limit: usize,
    ) -> impl Future<Output = TursoResult<Vec<String>>> + Send;

    /// 3-signal hybrid recall: `FTS5-BM25 ∪ vector ∪ graph → RRF`.
    ///
    /// Seeds graph expansion from the top vector+FTS hits (depth 2).
    ///
    /// # Errors
    /// Returns [`TursoError`] when any signal query fails.
    fn search_hybrid(
        &self,
        project: &str,
        query: &str,
        limit: usize,
    ) -> impl Future<Output = TursoResult<Vec<BrainHit>>> + Send;

    /// Backfill NULL embeddings in batches; returns rows updated.
    ///
    /// # Errors
    /// Returns [`TursoError`] on SQL failure.
    fn backfill_embeddings(&self, batch: usize) -> impl Future<Output = TursoResult<usize>> + Send;
}

impl VectorStore for TursoStore {
    async fn vector_rank(
        &self,
        project: &str,
        query: &str,
        limit: usize,
    ) -> TursoResult<Vec<String>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        let qv = embed_texts(&[query.to_owned()])?
            .into_iter()
            .next()
            .ok_or_else(|| TursoError::Mapping("vector_rank empty query vec".into()))?;

        // Prefer libsql ANN — fallback to brute scan if extension absent.
        let ann_sql = "SELECT entry_key FROM vector_top_k('idx_memories_embedding', vector32(?1), ?2) JOIN memories ON memories.rowid = id WHERE project = ?3";
        let vec_json = format!(
            "[{}]",
            qv.iter()
                .map(|v| format!("{v:.6}"))
                .collect::<Vec<_>>()
                .join(",")
        );
        #[expect(
            clippy::cast_possible_wrap,
            reason = "limit is bounded search cap; far below i64::MAX"
        )]
        let limit_i = limit as i64;
        let attempt = self
            .exec_params(
                ann_sql,
                libsql::params![vec_json.clone(), limit_i, project.to_owned()],
            )
            .await;
        if let Ok(mut rows) = attempt {
            let mut ann_out = Vec::new();
            while let Some(row) = rows.next().await.map_err(TursoError::from)? {
                if let Ok(key) = row.get::<String>(0) {
                    ann_out.push(key);
                }
            }
            if !ann_out.is_empty() {
                return Ok(ann_out);
            }
        }
        brute_rank(self, project, &qv, limit).await
    }

    async fn search_hybrid(
        &self,
        project: &str,
        query: &str,
        limit: usize,
    ) -> TursoResult<Vec<BrainHit>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        // Signal 1: per-category BM25 rank lists.
        let mut ranked: Vec<Vec<String>> = Vec::new();
        for cat in CORPUS_CATEGORIES {
            ranked.push(self.fts_rank_category(project, cat, query, limit).await?);
        }
        // Signal 2: semantic vector rank (global, filtered by project).
        let vec_list = self.vector_rank(project, query, limit).await?;
        if !vec_list.is_empty() {
            ranked.push(vec_list);
        }
        // Signal 3: graph closure seeded from top BM25 + vector hits.
        let mut seeds: Vec<String> = ranked
            .iter()
            .flat_map(|l| l.iter().take(3).cloned())
            .collect();
        seeds.sort_unstable();
        seeds.dedup();
        seeds.truncate(10);
        let graph_list = graph_expand(self, project, &seeds, 2).await?;
        if !graph_list.is_empty() {
            ranked.push(graph_list);
        }
        if ranked.is_empty() {
            return Ok(Vec::new());
        }
        let refs: Vec<&[String]> = ranked.iter().map(Vec::as_slice).collect();
        Ok(rrf_fuse(&refs, RRF_K)
            .into_iter()
            .take(limit)
            .map(|(id, score)| BrainHit { id, score })
            .collect())
    }

    async fn backfill_embeddings(&self, batch: usize) -> TursoResult<usize> {
        let batch = if batch == 0 { 50 } else { batch };
        let mut total = 0usize;
        loop {
            let sql = format!(
                "SELECT project, category, entry_key, title, content \
                 FROM memories WHERE embedding IS NULL LIMIT {batch}"
            );
            let mut rows = self.exec_params(&sql, ()).await?;
            let mut chunk: Vec<(String, String, String, String, String)> = Vec::new();
            while let Some(row) = rows.next().await.map_err(TursoError::from)? {
                let proj: String = row.get(0).map_err(TursoError::from)?;
                let cat: String = row.get(1).map_err(TursoError::from)?;
                let ekey: String = row.get(2).map_err(TursoError::from)?;
                let title: String = row.get(3).map_err(TursoError::from)?;
                let content: String = row.get(4).map_err(TursoError::from)?;
                chunk.push((proj, cat, ekey, title, content));
            }
            if chunk.is_empty() {
                break;
            }
            let texts: Vec<String> = chunk
                .iter()
                .map(|(_, _, _, title, content)| {
                    if content.is_empty() {
                        title.clone()
                    } else {
                        format!("{title}\n{content}")
                    }
                })
                .collect();
            let vecs = embed_texts(&texts)?;
            let chunk_len = chunk.len();
            for (idx, (proj, cat, ekey, _, _)) in chunk.into_iter().enumerate() {
                let Some(vec) = vecs.get(idx) else { continue };
                let blob = serialize_embedding(vec);
                self.conn
                    .execute(
                        "UPDATE memories SET embedding = ?1 \
                         WHERE project = ?2 AND category = ?3 AND entry_key = ?4",
                        libsql::params![blob, proj, cat, ekey],
                    )
                    .await
                    .map_err(TursoError::from)?;
                total = total.saturating_add(1);
            }
            if chunk_len < batch {
                break;
            }
        }
        Ok(total)
    }
}

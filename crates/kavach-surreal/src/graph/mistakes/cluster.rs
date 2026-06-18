// split: utility graph clustering module, not a handler
use crate::error::{Error, Result};
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::{RecordId, SurrealValue};

pub const COSINE_THRESHOLD: f32 = 0.85;
const KNN_K: i64 = 1;

#[derive(SurrealValue)]
struct IdRow {
    id: RecordId,
}

#[derive(SurrealValue)]
struct Hit {
    id: RecordId,
    score: f32,
}

/// Maps an event to a clustered anti-pattern.
///
/// # Errors
/// Propagates `Error::Surreal` when database queries fail.
pub async fn cluster_event_to_pattern(
    db: &Surreal<Db>,
    event_id: &RecordId,
    event_embedding: &[f32],
    fallback_gate: &str,
    fallback_correct_action: &str,
) -> Result<RecordId> {
    let neighbor = nearest_anti_pattern(db, event_embedding).await?;
    let pattern_id = match neighbor {
        Some((id, score)) if score >= COSINE_THRESHOLD => id,
        Some((_id, _score)) => {
            super::pattern::upsert_anti_pattern(
                db,
                &derive_pattern_name(fallback_gate, fallback_correct_action),
                fallback_gate,
                fallback_correct_action,
                event_embedding,
            )
            .await?
        }
        None => {
            super::pattern::upsert_anti_pattern(
                db,
                &derive_pattern_name(fallback_gate, fallback_correct_action),
                fallback_gate,
                fallback_correct_action,
                event_embedding,
            )
            .await?
        }
    };
    let q = "RELATE $src->instance_of->$tgt SET weight = 1.0 RETURN id";
    let mut resp = db
        .query(q)
        .bind(("src", event_id.clone()))
        .bind(("tgt", pattern_id.clone()))
        .await?;
    let _: Option<IdRow> = resp.take(0)?;
    Ok(pattern_id)
}

fn derive_pattern_name(gate: &str, correct_action: &str) -> String {
    let hash = blake3::hash(correct_action.as_bytes());
    let hex = hash.to_hex();
    let short: String = hex.chars().take(8).collect();
    let mut name = String::with_capacity(gate.len().saturating_add(14));
    name.push_str("anti.");
    name.push_str(gate);
    name.push('.');
    name.push_str(&short);
    name
}

async fn nearest_anti_pattern(
    db: &Surreal<Db>,
    embedding: &[f32],
) -> Result<Option<(RecordId, f32)>> {
    let q = "SELECT id, vector::similarity::cosine(embedding, $q) AS score \
             FROM entity WHERE entity_type = 'anti_pattern' AND embedding IS NOT NONE \
             ORDER BY score DESC LIMIT $k";
    let mut resp = db
        .query(q)
        .bind(("q", embedding.to_vec()))
        .bind(("k", KNN_K))
        .await?;
    let hits: Vec<Hit> = resp
        .take(0)
        .map_err(|e| Error::Migration(format!("knn: {e}")))?;
    Ok(hits.into_iter().next().map(|h| (h.id, h.score)))
}

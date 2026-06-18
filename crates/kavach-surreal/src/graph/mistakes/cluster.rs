// split: utility graph clustering module, not a handler
//
// Vector-free clustering: a mistake_event is mapped to its anti_pattern by a
// DETERMINISTIC content key — `anti.<gate>.<blake3(correct_action)[..8]>`. Two
// events with the same gate + correct_action upsert to the SAME node (exact-key
// dedup via the DAG). This replaces the former cosine k-NN over ONNX embeddings:
// the embedder is gone (decision/onnx-removal-dag-rlaif-only), and the only
// thing the k-NN bought — fuzzy merge of differently-worded same mistakes — is
// intentionally dropped. Recurrence is still counted via inbound instance_of
// edges; RLAIF grading still rides on the node.
use crate::error::{Error, Result};
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use surrealdb_types::{RecordId, SurrealValue};

#[derive(SurrealValue)]
struct IdRow {
    id: RecordId,
}

/// Maps an event to a clustered anti-pattern by content key (no vectors).
///
/// # Errors
/// Propagates `Error::Surreal` when database queries fail.
pub async fn cluster_event_to_pattern(
    db: &Surreal<Db>,
    event_id: &RecordId,
    gate: &str,
    correct_action: &str,
) -> Result<RecordId> {
    let pattern_id = super::pattern::upsert_anti_pattern(
        db,
        &derive_pattern_name(gate, correct_action),
        gate,
        correct_action,
    )
    .await?;
    let q = "RELATE $src->instance_of->$tgt SET weight = 1.0 RETURN id";
    let mut resp = db
        .query(q)
        .bind(("src", event_id.clone()))
        .bind(("tgt", pattern_id.clone()))
        .await?;
    let row: Option<IdRow> = resp.take(0)?;
    row.map(|_| pattern_id)
        .ok_or_else(|| Error::RecordNotFound("instance_of relate empty".into()))
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

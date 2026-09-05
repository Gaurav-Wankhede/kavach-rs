use crate::turso::TursoStore;
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

/// Maps an event to a clustered anti-pattern by content key (no vectors).
///
/// # Errors
/// Propagates `Error::Turso` when database queries fail.
pub async fn cluster_event_to_pattern(
    db: &TursoStore,
    event_id: &str,
    gate: &str,
    correct_action: &str,
) -> Result<String> {
    let pattern_id = super::pattern::upsert_anti_pattern(
        db,
        &derive_pattern_name(gate, correct_action),
        gate,
        correct_action,
    )
    .await?;
    // Idempotent edge: skip when the instance_of edge already exists.
    let mut existing_resp = db
        .exec_params(
            "SELECT 1 FROM entity_edges WHERE from_id = ?1 AND rel_type = 'instance_of' AND to_id = ?2 LIMIT 1",
            libsql::params![event_id.to_owned(), pattern_id.clone()],
        )
        .await?;
    if existing_resp.next().await?.is_some() {
        return Ok(pattern_id);
    }
    let now = crate::turso::now_rfc3339();
    let mut resp = db
        .exec_params(
            "INSERT INTO entity_edges (from_id, rel_type, to_id, weight, created_at) \
             VALUES (?1, 'instance_of', ?2, 1.0, ?3) \
             ON CONFLICT(from_id, rel_type, to_id) DO UPDATE SET weight = 1.0 \
             RETURNING from_id",
            libsql::params![event_id.to_owned(), pattern_id.clone(), now],
        )
        .await?;
    match resp.next().await? {
        Some(_) => Ok(pattern_id),
        None => Err(Error::RecordNotFound("instance_of relate empty".into())),
    }
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

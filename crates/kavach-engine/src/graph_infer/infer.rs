//! The single inference rule: row B mentioning row A's `entry_key` (word-bounded)
//! emits a `references` edge B -> A.
use super::scan::mentions_key;
use super::types::{InferRow, InferredRel};

/// Minimum key length to consider for content mentions. Shorter keys produce
/// too many false-positive substring hits ("ab", "id", etc).
const MIN_KEY_LEN: usize = 4;

pub fn infer_relationships(rows: &[InferRow]) -> Vec<InferredRel> {
    let mut out: Vec<InferredRel> = Vec::new();
    if rows.is_empty() {
        return out;
    }
    let qnames: Vec<String> = rows.iter().map(qname).collect();
    for (i, src) in rows.iter().enumerate() {
        for (j, tgt) in rows.iter().enumerate() {
            if i == j || tgt.entry_key.len() < MIN_KEY_LEN {
                continue;
            }
            if mentions_key(&src.content, &tgt.entry_key) {
                let (Some(src_q), Some(tgt_q)) = (qnames.get(i), qnames.get(j)) else {
                    continue;
                };
                let edge = InferredRel {
                    source_qname: src_q.clone(),
                    rel: String::from("references"),
                    target_qname: tgt_q.clone(),
                };
                if !out.contains(&edge) {
                    out.push(edge);
                }
            }
        }
    }
    out
}

fn qname(r: &InferRow) -> String {
    format!("{}/{}/{}", r.project_slug, r.category, r.entry_key)
}

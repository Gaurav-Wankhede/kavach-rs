//! Entity list query handler.

use crate::error::internal;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;

use super::types::GraphQueryResult;

pub(super) async fn graph_query_list(
    ctx: &AppState,
    entity_type: Option<&str>,
    limit: usize,
) -> Result<GraphQueryResult, ErrorObjectOwned> {
    let entities = kavach_db::graph_list_entities(&ctx.db, entity_type)
        .await
        .map_err(|e| internal(e.to_string()))?;
    let total = entities.len();
    let shown = total.min(limit);
    let mut lines = Vec::with_capacity(shown);
    for e in entities.iter().take(limit) {
        let line = format_entity_line(e);
        lines.push(line);
    }
    Ok(GraphQueryResult {
        success: true,
        lines,
        total,
        shown,
        error: None,
    })
}

fn format_entity_line(e: &kavach_db::Entity) -> String {
    use std::fmt::Write as _;

    let id_str = e.id.as_ref().map_or_else(
        || "-".to_owned(),
        |id| {
            let mut s = String::with_capacity(32);
            let _ = write!(s, "{id:?}").ok();
            s
        },
    );
    let mut line = String::with_capacity(128);
    match &e.properties {
        Some(serde_json::Value::String(s)) if !s.is_empty() => {
            let _ = write!(line, "[{}] {} (id={}) {}", e.entity_type, e.name, id_str, s).ok();
        }
        Some(p) => {
            let _ = write!(line, "[{}] {} (id={}) {}", e.entity_type, e.name, id_str, p).ok();
        }
        None => {
            let _ = write!(line, "[{}] {} (id={})", e.entity_type, e.name, id_str).ok();
        }
    }
    line
}

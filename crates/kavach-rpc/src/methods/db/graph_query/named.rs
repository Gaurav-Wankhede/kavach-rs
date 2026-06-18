//! Named entity query handler.

use crate::error::internal;
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;

use super::types::GraphQueryResult;

pub(super) async fn graph_query_named(
    ctx: &AppState,
    etype: &str,
    n: &str,
) -> Result<GraphQueryResult, ErrorObjectOwned> {
    use std::fmt::Write as _;

    let entity = match kavach_surreal::graph_find_entity(&ctx.db, etype, n).await {
        Ok(Some(e)) => e,
        Ok(None) => {
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "capacity estimation, operands bounded"
            )]
            let mut msg = String::with_capacity(etype.len() + n.len() + 26);
            #[expect(clippy::expect_used, reason = "write to String never fails")]
            write!(msg, "entity not found: {etype}/{n}").expect("write to String never fails");
            return Ok(GraphQueryResult {
                success: false,
                lines: vec![],
                total: 0,
                shown: 0,
                error: Some(msg),
            });
        }
        Err(e) => return Err(internal(e.to_string())),
    };
    let Some(entity_id) = entity.id else {
        return Ok(GraphQueryResult {
            success: false,
            lines: vec![],
            total: 0,
            shown: 0,
            error: Some("entity has no id".to_owned()),
        });
    };
    let rows = kavach_surreal::graph_get_related(&ctx.db, &entity_id, 200)
        .await
        .map_err(|e| internal(e.to_string()))?;
    let mut lines = Vec::with_capacity(rows.len().saturating_add(2));
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "capacity estimation, operands bounded"
    )]
    let mut header = String::with_capacity(etype.len() + n.len() + 20);
    #[expect(clippy::expect_used, reason = "write to String never fails")]
    write!(header, "[{etype}] {n} (id={entity_id:?})").expect("write to String never fails");
    lines.push(header);
    if rows.is_empty() {
        lines.push("  (no edges)".to_owned());
    } else {
        for r in &rows {
            let mut line = String::with_capacity(64);
            #[expect(clippy::expect_used, reason = "write to String never fails")]
            write!(
                line,
                "  ──{}──► [{}] {}",
                r.rel_type, r.target.entity_type, r.target.name
            )
            .expect("write to String never fails");
            lines.push(line);
        }
    }
    let shown = lines.len();
    Ok(GraphQueryResult {
        success: true,
        lines,
        total: shown,
        shown,
        error: None,
    })
}

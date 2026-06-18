//! Knowledge graph — `db.graph_fetch` rendered with Cytoscape.js. The page ships
//! the canvas + bootstrap script; the browser fetches node/edge JSON from
//! `/knowledge/data` and feeds it to the layout.

use axum::Json;
use axum::response::Html;
use serde_json::{Value, json};

use crate::error::error_panel;
use crate::layout::{heading, shell};
use crate::rpc::call;

/// `GET /knowledge` — the Cytoscape canvas + loader script.
pub async fn page() -> Html<String> {
    let inner = maud::html! {
        (heading("Knowledge Graph"))
        div #cy {}
        script src="/static/cytoscape.min.js" {}
        script src="/static/knowledge.js" {}
    };
    Html(shell("/knowledge", None, inner).into_string())
}

/// `GET /knowledge/data` — `{nodes, edges}` JSON for the Cytoscape layout. On
/// RPC failure returns a JSON error object the loader script renders inline.
pub async fn data() -> Json<Value> {
    match call::<_, Value>("db.graph_fetch", json!({ "limit": 300 })).await {
        Ok(v) => Json(v),
        Err(e) => Json(json!({
            "success": false,
            "nodes": [],
            "edges": [],
            "error": error_panel(&e).into_string(),
        })),
    }
}

//! Shared rendering of RPC failures into user-facing HTML.

use axum::response::Html;
use maud::{Markup, html};

use crate::rpc::RpcError;

/// Render an RPC error as an inline panel. A daemon-offline failure gets a
/// distinct, actionable message (how to start the daemon); any other failure
/// shows the error text.
#[must_use]
pub fn error_panel(e: &RpcError) -> Markup {
    if e.is_offline() {
        html! {
            div.panel.panel-offline {
                h2 { "kavach-rpc daemon is not running" }
                p { "The web UI reads everything through the daemon over its Unix socket." }
                pre { "kavach daemon install\nlaunchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/ai.shared.kavach-rpc.plist" }
            }
        }
    } else {
        html! {
            div.panel.panel-error {
                h2 { "Request failed" }
                pre { (e.to_string()) }
            }
        }
    }
}

/// Collapse a `Result<Markup, RpcError>` into HTML — the body on success, the
/// error panel on failure. Keeps every handler a one-liner at the call site.
pub fn render(result: Result<Markup, RpcError>) -> Html<String> {
    match result {
        Ok(body) => Html(body.into_string()),
        Err(e) => Html(error_panel(&e).into_string()),
    }
}

//! kavach-web — server-rendered HTMX UI for the Kavach memory store.
//!
//! Runs an axum server on `127.0.0.1:<port>` (default 7777, unprivileged). It is
//! a thin ws client of the standalone `surreal start` server; it never opens the
//! `RocksDB` store directly. Replaces the removed Dioxus desktop GUI.

pub mod error;
pub mod layout;
pub mod orchestrate;
pub mod pages;
pub mod rpc;
pub mod sse;

use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;

use axum::Router;
use axum::routing::{get, post};
use tower_http::services::ServeDir;

/// Default loopback port for the web UI. Unprivileged (>1024) so it binds
/// without root; 777 is privileged and fails with `EACCES` on macOS/Linux.
pub const DEFAULT_PORT: u16 = 7777;

/// Build the axum router with all page, fragment, write, and SSE routes plus the
/// `/static` asset mount.
pub fn router() -> Router {
    let static_dir = asset_dir();
    Router::new()
        .route("/", get(pages::projects::page))
        .route("/roadmap", get(pages::roadmap::page))
        .route("/roadmap/fragment", get(pages::roadmap::fragment))
        .route("/kanban", get(pages::kanban::page))
        .route("/kanban/fragment", get(pages::kanban::fragment))
        .route("/decisions", get(pages::decisions::page))
        .route("/decisions/fragment", get(pages::decisions::fragment))
        .route("/knowledge", get(pages::knowledge::page))
        .route("/knowledge/data", get(pages::knowledge::data))
        .route("/concepts", get(pages::concepts::page))
        .route("/concepts/fragment", get(pages::concepts::fragment))
        .route("/concepts/add", post(pages::concepts::add))
        .route("/concepts/search", get(pages::concepts::search))
        .route("/citations", get(pages::citations::page))
        .route("/citations/fragment", get(pages::citations::fragment))
        .route("/citations/add", post(pages::citations::add))
        .route("/mistakes", get(pages::mistakes::page))
        .route("/mistakes/lookup", get(pages::mistakes::lookup))
        .route("/runs", get(pages::runs::page))
        .route("/runs/fragment", get(pages::runs::fragment))
        .route("/runs/cancel", post(pages::runs::cancel))
        .route("/runs/spawn", post(pages::runs::spawn))
        .route("/entries/edit", get(pages::editor::edit))
        .route("/entries/save", post(pages::editor::save))
        .route("/entries/status", post(pages::editor::status))
        .route("/events", get(sse::events))
        .route(
            "/v1/chat/completions",
            post(orchestrate::chat_completions),
        )
        .nest_service("/static", ServeDir::new(static_dir))
}

/// Resolve the static asset directory shipped alongside the crate.
fn asset_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

/// Run the web server, blocking until shutdown. Builds its own Tokio runtime so
/// the synchronous CLI dispatch (`kavach web`) can call it directly.
///
/// # Errors
/// Returns an error if the runtime fails to build or the listener cannot bind.
pub fn serve(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(async move {
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
        let listener = tokio::net::TcpListener::bind(addr).await?;
        tracing::info!(%addr, "kavach-web listening — open http://{addr}");
        axum::serve(listener, router()).await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    })
}

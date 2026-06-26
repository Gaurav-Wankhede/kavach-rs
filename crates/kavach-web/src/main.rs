//! `kavach-web` binary — launches the HTMX UI server.
//!
//! Usage: `kavach-web [PORT]` (default 7777). Thin ws client of the standalone
//! `surreal start` server; start it via `kavach servers up` if pages show the
//! offline panel.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let port = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(kavach_web::DEFAULT_PORT);

    kavach_web::serve(port)
}

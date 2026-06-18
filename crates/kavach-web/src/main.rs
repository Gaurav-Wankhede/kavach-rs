//! `kavach-web` binary — launches the HTMX UI server.
//!
//! Usage: `kavach-web [PORT]` (default 777). Reads everything through the
//! running kavach-rpc daemon; start it first with `kavach daemon install` +
//! launchctl bootstrap if the pages show the offline panel.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let port = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(kavach_web::DEFAULT_PORT);

    kavach_web::serve(port)
}

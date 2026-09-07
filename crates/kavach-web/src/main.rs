//! `kavach-web` binary — launches the HTMX UI server.
//!
//! Usage: `kavach-web [PORT]` (default 7777). Thin ws client of the standalone
//! `surreal start` server; start it via `kavach servers up` if pages show the
//! offline panel.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filter = match tracing_subscriber::EnvFilter::try_from_default_env() {
        Ok(f) => f,
        Err(_) => tracing_subscriber::EnvFilter::new("info"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let port = match std::env::args().nth(1) {
        Some(s) => match s.parse::<u16>() {
            Ok(p) => p,
            Err(_) => kavach_web::DEFAULT_PORT,
        },
        None => kavach_web::DEFAULT_PORT,
    };

    kavach_web::serve(port)
}

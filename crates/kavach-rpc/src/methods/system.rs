// split: intentional - system-level RPC methods (health, schema_apply, shutdown)
use crate::state::AppState;
use jsonrpsee::types::ErrorObjectOwned;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC response DTO; server-constructed"
)]
pub struct HealthResponse {
    pub status: &'static str,
    pub started_at: String,
    pub uptime_seconds: i64,
    pub pid: u32,
}

/// Get current server health status.
///
/// # Errors
/// Never returns an error; always succeeds.
#[expect(
    clippy::unused_async,
    reason = "registered as an async RPC handler; signature uniformity with other system.* methods"
)]
pub async fn health(state: &AppState) -> Result<HealthResponse, ErrorObjectOwned> {
    let now = chrono::Utc::now();
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "DateTime subtraction is safe, produces signed duration"
    )]
    let uptime = (now - state.started_at).num_seconds();
    Ok(HealthResponse {
        status: "ok",
        started_at: state.started_at.to_rfc3339(),
        uptime_seconds: uptime,
        pid: std::process::id(),
    })
}



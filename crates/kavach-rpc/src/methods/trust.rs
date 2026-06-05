// split: intentional - cohesive trust RPC group (classify + should_surface)
// JSON-RPC method handlers exposing kavach_patterns::trust_score over the existing socket.
// SOURCE: https://docs.rs/jsonrpsee/latest/jsonrpsee/struct.RpcModule.html
use jsonrpsee::types::ErrorObjectOwned;
use kavach_patterns::trust_score::{self, AdvisoryTier, TrustInputs, TrustLevel};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "JSON-RPC wire DTO, constructed at handler boundary"
)]
pub struct ClassifyParams {
    pub session_count: u32,
    pub accepted_advisories: u32,
    pub rejected_advisories: u32,
    pub p0_blocks_in_last_30_sessions: u32,
}

#[derive(Debug, Clone, Serialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "JSON-RPC wire DTO, constructed at handler boundary"
)]
pub struct ClassifyResult {
    pub level: String,
    pub suppresses_p1: bool,
    pub suppresses_p2: bool,
}

const fn level_str(l: TrustLevel) -> &'static str {
    match l {
        TrustLevel::Probationary => "probationary",
        TrustLevel::Developing => "developing",
        TrustLevel::Established => "established",
        TrustLevel::Mature => "mature",
    }
}

/// Classifies a trust level based on advisory counts and session history.
///
/// # Errors
///
/// This function returns an error if classification fails, though the current implementation
/// is infallible.
#[expect(
    clippy::unused_async,
    reason = "JSON-RPC handler signature requires async"
)]
pub async fn classify(
    _state: &AppState,
    params: ClassifyParams,
) -> Result<ClassifyResult, ErrorObjectOwned> {
    let inputs = TrustInputs {
        session_count: params.session_count,
        accepted_advisories: params.accepted_advisories,
        rejected_advisories: params.rejected_advisories,
        p0_blocks_in_last_30_sessions: params.p0_blocks_in_last_30_sessions,
    };
    let level = trust_score::classify(inputs);
    Ok(ClassifyResult {
        level: level_str(level).into(),
        suppresses_p1: level.suppresses_p1(),
        suppresses_p2: level.suppresses_p2(),
    })
}

#[derive(Debug, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "JSON-RPC wire DTO, constructed at handler boundary"
)]
pub struct ShouldSurfaceParams {
    pub tier: String,
    pub level: String,
}

#[derive(Debug, Clone, Serialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "JSON-RPC wire DTO, constructed at handler boundary"
)]
pub struct ShouldSurfaceResult {
    pub surface: bool,
}

/// Determines whether an advisory should be surfaced based on tier and trust level.
///
/// # Errors
///
/// Returns an error if the `tier` or `level` parameters contain invalid values.
#[expect(
    clippy::unused_async,
    reason = "JSON-RPC handler signature requires async"
)]
pub async fn should_surface(
    _state: &AppState,
    params: ShouldSurfaceParams,
) -> Result<ShouldSurfaceResult, ErrorObjectOwned> {
    let tier = match params.tier.as_str() {
        "p0" | "P0" | "P0Block" => AdvisoryTier::P0Block,
        "p1" | "P1" | "P1Advisory" => AdvisoryTier::P1Advisory,
        "p2" | "P2" | "P2Warning" => AdvisoryTier::P2Warning,
        other => {
            return Err(ErrorObjectOwned::owned(
                -32602,
                format!("invalid tier: {other}"),
                None::<()>,
            ));
        }
    };
    let level = match params.level.as_str() {
        "probationary" | "Probationary" => TrustLevel::Probationary,
        "developing" | "Developing" => TrustLevel::Developing,
        "established" | "Established" => TrustLevel::Established,
        "mature" | "Mature" => TrustLevel::Mature,
        other => {
            return Err(ErrorObjectOwned::owned(
                -32602,
                format!("invalid level: {other}"),
                None::<()>,
            ));
        }
    };
    Ok(ShouldSurfaceResult {
        surface: trust_score::should_surface(tier, level),
    })
}

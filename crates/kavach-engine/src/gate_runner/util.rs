//! Shared dispatch helper: run an infallible gate side-effect and report success.
use crate::error::EngineError;

/// Run an infallible gate side-effect and report success.
#[expect(
    clippy::unnecessary_wraps,
    reason = "uniform Result<(), EngineError> arm type so infallible gates compose in the same match as fallible ones"
)]
pub(super) fn ok(run: impl FnOnce()) -> Result<(), EngineError> {
    run();
    Ok(())
}

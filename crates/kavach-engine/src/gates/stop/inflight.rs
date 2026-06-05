//! Yield-group guards: the Stop must YIELD (not block) while async work is in
//! flight — background tasks, session crons, active teammates, or an open bulk
//! sweep. Each is its own single-responsibility child; this hub re-exports them.

mod background;
mod bulk_sweep;
mod teammate;

pub(crate) use background::check as background;
pub(crate) use bulk_sweep::check as bulk_sweep;
pub(crate) use teammate::check as teammate;

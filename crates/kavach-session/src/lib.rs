// `SessionState` inherent methods are intentionally split across modules
// (create/load/save/serialize/markers/phase/...) for file-size locality, so the
// crate has many `impl SessionState` blocks by design. One crate-level expect
// covers them all; per-file suppressions are intentionally NOT used.
#![expect(
    clippy::multiple_inherent_impl,
    reason = "SessionState impl deliberately split across modules for file-size locality"
)]
#![expect(
    clippy::redundant_pub_crate,
    reason = "nursery lint conflicts with workspace unreachable_pub=deny: pub(crate) fns in private modules satisfy unreachable_pub; redundant_pub_crate's pub suggestion would re-trigger it"
)]

mod compact;
mod create;
mod enforcement;
mod get_or_create;
mod intent;
mod load;
mod markers;
pub mod mistake_ledger;
pub mod mistake_ledger_graph;
mod modules;
mod parse;
pub mod paths;
mod phase;
mod save;
mod serialize;
mod serialize_enforcement;
mod serialize_extras;
pub mod state;
mod state_default;
mod subagent;
mod subset;
mod task;
mod team_tracking;
mod turn_shadow;
pub use turn_shadow::{RelayFlush, RewardOutcome};

pub use get_or_create::{
    get_or_create_session, get_or_create_session_for, set_session_context,
};
pub use load::{load_session_state, load_session_state_for, parse_ini_str};
pub use mistake_ledger::{Mistake, record as record_mistake};
pub use paths::{canonicalize_iteration_path, memory_dir, state_dir, state_path, stm_path};
pub use state::SessionState;
pub use subagent::SubagentBlast;
pub use subset::{SessionFlags, SessionIdentity, SessionTracking};

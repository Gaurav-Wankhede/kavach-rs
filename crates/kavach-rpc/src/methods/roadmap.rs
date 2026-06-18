// split: intentional - cohesive roadmap RPC group (entry_status + next_open_task)
// JSON-RPC method handlers for roadmap kanban inspection.
// Used by the kavach-engine stop gate to read open kanban cards without spawning tokio.

pub mod backlog;
pub mod card_mutation;
pub mod dispatch;
pub mod query;
pub mod readiness;
pub mod types;

pub use backlog::promote_next_backlog;
pub use card_mutation::{claim_card, verify_card};
pub use dispatch::{next_open_hunt, next_open_task, open_set_census, ready_set};
pub use query::{entry_status, list_done_cards, list_in_progress_cards, list_titles};
pub use types::{
    ClaimCardParams, ClaimCardResult, EntryStatusParams, EntryStatusResult, ListTitlesParams,
    NextOpenTaskParams, NextTaskResult, OpenSetCensus, TitleRow, VerifyCardResult,
};

#[cfg(test)]
#[path = "roadmap/readiness_test.rs"]
mod readiness_tests;

#[cfg(test)]
#[path = "roadmap/backlog_test.rs"]
mod backlog_tests;

#[cfg(test)]
#[path = "roadmap/card_mutation_test.rs"]
mod card_mutation_tests;

//! Intent-gate context builders, grouped by concern: `directives` (intent-keyed
//! reminders + RCA protocol + agent dispatch), `db_query` (status-prompt kanban
//! requirement + session isolation), `research` (topic extraction).
mod db_query;
mod directives;
mod research;

#[cfg(test)]
mod tests;

pub(crate) use db_query::append_db_query_required;
pub(crate) use directives::{
    append_agent_dispatch, append_forbidden, append_memory_db, append_root_cause_protocol,
    append_verify_existing,
};
pub(crate) use research::extract_research_topic;

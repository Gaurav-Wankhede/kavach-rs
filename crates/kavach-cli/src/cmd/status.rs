use std::io::{self, Write};

use crate::cli::{KAVACH_BUILD_TIMESTAMP, KAVACH_GIT_SHA};

/// `kavach status` — print binary build identity, session state, and enforcement flags.
pub(super) fn run() -> i32 {
    let session = kavach_session::get_or_create_session();
    let toon = session.to_compact();

    let research = if session.research_done {
        "DONE"
    } else {
        "PENDING"
    };
    let memory = if session.memory_queried {
        "DONE"
    } else {
        "PENDING"
    };
    let phase = &session.context_phase;

    let output = format!(
        "[BINARY]\n\
         build: {KAVACH_BUILD_TIMESTAMP}\n\
         git: {KAVACH_GIT_SHA}\n\
         {toon}\
         [ENFORCEMENT]\n\
         research: {research}\n\
         memory: {memory}\n\
         turn_count: {}\n\
         context_phase: {phase}\n\
         active_subagents: {}\n\
         files_modified: {}\n",
        session.turn_count,
        session.active_subagents,
        session.files_modified.len(),
    );

    drop(io::stdout().lock().write_all(output.as_bytes()));
    0
}

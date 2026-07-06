//! HARD BLOCK: Force memory operations through kavach-db, not MEMORY.md.
//!
//! When the agent writes to memory files (MEMORY.md, /memory/*.md),
//! this guard blocks the write and directs to kavach db commands.
//! kavach-db (`SurrealDB`) is the permanent store — MEMORY.md is session cache.

/// Check if a file path is a memory file that should use kavach-db.
/// Returns false for agent-memory paths — subagents write structured
/// per-agent memory under ~/.claude/agent-memory/<name>/ and those
/// writes are intentional, not kavach-db bypass attempts.
pub(crate) fn is_memory_file(file_path: &str) -> bool {
    if file_path.contains("agent-memory/") {
        return false;
    }
    file_path.contains("/memory/")
        || file_path.ends_with("MEMORY.md")
        || file_path.ends_with("/memory.md")
}

/// Generate the block message for memory file writes.
pub(crate) fn block_message(file_path: &str) -> String {
    format!(
        "[MEMORY_DB_POLICY] Writing to `{file_path}` bypasses the permanent store.\n\
         \n\
         kavach-db (SurrealDB) is the PERMANENT store — MEMORY.md is session cache only.\n\
         \n\
         USE THESE COMMANDS INSTEAD:\n\
         \n\
         # Write a memory entry\n\
         kavach db write --project <slug> --category <cat> --key <key> --title <title>\n\
         \n\
         # Query existing memories\n\
         kavach db query --project <slug>\n\
         kavach db query --project <slug> --category decision\n\
         \n\
         # Sync session state to database\n\
         kavach db sync\n\
         \n\
         CATEGORIES: decision, pattern, research, architecture, debug, config\n\
         \n\
         Use `kavach db write` to persist memories — MEMORY.md is session cache only.\n\
         EXCEPTION: Only Claude Code auto-memory (system-initiated) may write MEMORY.md."
    )
}

#[cfg(test)]
#[path = "memory_write_guard_test.rs"]
mod tests;

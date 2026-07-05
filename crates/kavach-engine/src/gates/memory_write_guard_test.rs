use super::*;

#[test]
fn memory_md_detected() {
    assert!(is_memory_file(
        "/Users/x/.claude/projects/foo/memory/MEMORY.md"
    ));
}

#[test]
fn memory_dir_detected() {
    assert!(is_memory_file(
        "/Users/x/.claude/projects/foo/memory/topic.md"
    ));
}

#[test]
fn normal_file_passes() {
    assert!(!is_memory_file("/Users/x/project/src/main.rs"));
}

#[test]
fn agent_memory_dir_exempt() {
    assert!(!is_memory_file(
        "/Users/x/.claude/agent-memory/backend-engineer/user_role.md"
    ));
    assert!(!is_memory_file(
        "/Users/x/.claude/agent-memory/frontend-engineer/MEMORY.md"
    ));
}

#[test]
fn block_message_has_commands() {
    let msg = block_message("MEMORY.md");
    assert!(msg.contains("kavach db write"));
    assert!(msg.contains("kavach db query"));
    assert!(msg.contains("[MEMORY_DB_POLICY]"));
}

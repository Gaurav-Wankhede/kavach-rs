use super::*;

fn session_with_reads(n: usize) -> SessionState {
    let mut s = SessionState::default();
    for _ in 0..n {
        record_tool_call(&mut s, "Read", "/src/main.rs");
    }
    s
}

#[test]
fn first_read_passes() {
    let s = session_with_reads(1);
    assert!(check_duplicate_tool(&s, "Read", "/src/main.rs").is_none());
}

#[test]
fn duplicate_read_warns() {
    let s = session_with_reads(2);
    let r = check_duplicate_tool(&s, "Read", "/src/main.rs");
    assert!(r.is_some());
    assert!(r.unwrap().contains("DUPLICATE_TOOL"));
}

#[test]
fn different_files_pass() {
    let mut s = SessionState::default();
    record_tool_call(&mut s, "Read", "/src/a.rs");
    record_tool_call(&mut s, "Read", "/src/b.rs");
    assert!(check_duplicate_tool(&s, "Read", "/src/a.rs").is_none());
}

#[test]
fn empty_target_passes() {
    let s = SessionState::default();
    assert!(check_duplicate_tool(&s, "Read", "").is_none());
}

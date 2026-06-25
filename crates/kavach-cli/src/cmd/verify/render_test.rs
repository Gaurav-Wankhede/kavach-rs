use super::{cargo_cmd, stderr_head};

#[test]
fn cmd_with_crate_shows_the_p_flag() {
    assert_eq!(cargo_cmd(&["check"], Some("chat-service")), "cargo check -p chat-service");
}

#[test]
fn cmd_without_crate_is_workspace() {
    assert_eq!(cargo_cmd(&["nextest", "run"], None), "cargo nextest run");
}

#[test]
fn stderr_head_takes_n_nonblank_lines() {
    let s = "error one\n\n  \nerror two\nerror three";
    assert_eq!(stderr_head(s, 2), "error one\nerror two");
}

#[test]
fn stderr_head_empty_input_is_empty() {
    assert_eq!(stderr_head("   \n\n", 5), "");
}

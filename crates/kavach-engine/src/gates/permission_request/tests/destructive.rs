//! Destructive-command detection coverage incl. safe-absolute-path negatives.
use crate::gates::permission_request::destructive::{destructive_match, is_destructive_command};

#[test]
fn test_destructive() {
    assert!(is_destructive_command("rm -rf /"));
    assert!(is_destructive_command("rm -rf /*"));
    assert!(is_destructive_command("DROP TABLE users"));
    assert!(!is_destructive_command("rm -rf node_modules"));
    assert!(!is_destructive_command("npm test"));
}

#[test]
fn test_destructive_safe_absolute_paths() {
    assert!(!is_destructive_command("rm -rf /Users/foo/node_modules"));
    assert!(!is_destructive_command("rm -rf /home/user/.vite"));
    assert!(!is_destructive_command("rm -rf /tmp/build"));
    assert!(!is_destructive_command("rm -r /Users/foo/.next"));
}

#[test]
fn test_destructive_match_helper() {
    assert!(destructive_match("rm -rf /", "rm -rf /"));
    assert!(destructive_match("rm -rf /*", "rm -rf /"));
    assert!(!destructive_match("rm -rf /Users/foo", "rm -rf /"));
    // Non-slash patterns still use contains.
    assert!(destructive_match("drop table users", "drop table"));
}

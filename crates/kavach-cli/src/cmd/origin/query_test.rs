use super::run;
use std::path::Path;

#[test]
fn bad_json_returns_exit_2() {
    assert_eq!(run("{not json", Path::new("."), false), 2);
}

#[test]
fn missing_root_returns_exit_2() {
    assert_eq!(run("{}", Path::new("/nonexistent_xyz_123"), false), 2);
}

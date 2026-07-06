//! Tests for the pure self-update path helpers (no network/process spawn).
use super::{built_binary_path, install_dest_from};
use std::path::PathBuf;

#[test]
fn built_binary_path_joins_target_release_kavach() {
    let src = PathBuf::from("/tmp/kavach-update-1234");
    let got = built_binary_path(&src);
    assert_eq!(got, src.join("target").join("release").join("kavach"));
}

#[test]
fn install_dest_from_honors_override() {
    let got = install_dest_from(Some("/tmp/kavach-test-dest".to_owned()));
    assert_eq!(got, Ok(PathBuf::from("/tmp/kavach-test-dest/kavach")));
}

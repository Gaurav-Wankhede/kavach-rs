//! Tests for the pure self-update path helpers (no network/process spawn).
use super::{built_binary_path, install_dest};
use std::path::PathBuf;

#[test]
fn built_binary_path_joins_target_release_kavach() {
    let src = PathBuf::from("/tmp/kavach-update-1234");
    let got = built_binary_path(&src);
    assert_eq!(got, src.join("target").join("release").join("kavach"));
}

#[test]
fn install_dest_honors_kavach_install_dir_override() {
    // SAFETY: single-threaded test process; no concurrent env mutation.
    unsafe { std::env::set_var("KAVACH_INSTALL_DIR", "/tmp/kavach-test-dest") };
    let got = install_dest();
    unsafe { std::env::remove_var("KAVACH_INSTALL_DIR") };
    assert_eq!(got, Ok(PathBuf::from("/tmp/kavach-test-dest/kavach")));
}

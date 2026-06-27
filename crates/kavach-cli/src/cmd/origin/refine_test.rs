use super::on_path;

#[test]
fn cargo_is_on_path_in_dev() {
    assert!(on_path("cargo"), "cargo must resolve on PATH in a dev env");
}

#[test]
fn nonexistent_binary_is_not_on_path() {
    assert!(!on_path("kavach_definitely_not_a_real_binary_xyz"));
}

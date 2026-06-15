// Proves the ONNX-runtime dylib path is launch-independent: the conventional
// path is absolute, anchored under SharedAI/lib with the per-OS filename, so the
// install seam stages the dylib exactly where the daemon (cwd=/) will dlopen it.
use super::{conventional_dylib_path, os_dylib_name};

#[test]
fn conventional_path_is_absolute_and_under_lib() {
    let p = conventional_dylib_path();
    assert!(
        p.is_absolute(),
        "dylib path must be cwd-independent so the launchd daemon finds it, got {p:?}"
    );
    assert!(
        p.ends_with(os_dylib_name()),
        "path must end with the per-OS runtime filename, got {p:?}"
    );
    // The parent dir is the `lib/` sibling of the SharedAI DB — where the install
    // seam stages the binary.
    assert_eq!(
        p.parent().and_then(|d| d.file_name()),
        Some(std::ffi::OsStr::new("lib")),
        "dylib must live under <db-parent>/lib, got {p:?}"
    );
}

#[test]
fn os_dylib_name_matches_the_host_target() {
    let name = os_dylib_name();
    if cfg!(target_os = "macos") {
        assert_eq!(name, "libonnxruntime.dylib");
    } else if cfg!(target_os = "windows") {
        assert_eq!(name, "onnxruntime.dll");
    } else {
        assert_eq!(name, "libonnxruntime.so");
    }
}

#[test]
fn os_dylib_name_is_a_bare_filename_no_separators() {
    // The install seam joins this onto <db-parent>/lib — it must be a single
    // path component, never carry a separator that would escape the lib dir.
    let name = os_dylib_name();
    assert!(
        !name.contains('/') && !name.contains('\\'),
        "os dylib name must be a bare filename, got {name:?}"
    );
}

// Proves the model store dir is launch-independent: an explicit override is
// honored verbatim, and the default is an absolute path anchored under SharedAI
// (never the cwd-relative `.fastembed_cache` that left the daemon's model dark).
use super::resolve;
use std::path::PathBuf;

#[test]
fn explicit_override_is_honored_verbatim() {
    assert_eq!(
        resolve(Some("/tmp/kavach-fastembed".to_owned())),
        PathBuf::from("/tmp/kavach-fastembed")
    );
}

#[test]
fn relative_override_is_anchored_absolute() {
    // A relative FASTEMBED_CACHE_DIR must not slip the cwd-relative bug back in.
    let dir = resolve(Some("custom-cache".to_owned()));
    assert!(
        dir.is_absolute(),
        "relative override must be absolutized, got {dir:?}"
    );
    assert!(dir.ends_with("custom-cache"), "got {dir:?}");
}

#[test]
fn default_store_is_absolute_and_anchored() {
    let dir = resolve(None);
    assert!(
        dir.is_absolute(),
        "store must be cwd-independent so the launchd daemon finds it, got {dir:?}"
    );
    assert!(
        dir.ends_with("fastembed_cache"),
        "default anchors to the fastembed_cache dir, got {dir:?}"
    );
}

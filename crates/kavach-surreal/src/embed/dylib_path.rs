// ONNX-runtime dylib path resolution for the embedder — kept cwd-independent and
// launch-context-independent so the runtime is found on a FRESH machine, under
// `cargo test`, and from the launchd daemon (cwd=/) alike, not only when a
// hand-edited LaunchAgent plist happens to set ORT_DYLIB_PATH.
// SOURCE: decision.embedder-ort-dylib-in-process-resolver (Option B).
//
// ALGO: none — pure path resolution (env lookup + parent-dir join + per-OS
//   filename). TIME: O(1). SPACE: O(1). YEAR: 2026.
use std::path::PathBuf;

/// The env var `ort` (load-dynamic) reads to `dlopen` the ONNX Runtime.
///
/// The daemon-install seam injects this into the generated `LaunchAgent` so
/// provisioning is code-owned, not a hand-edited plist.
/// SOURCE: <https://ort.pyke.io/setup/linking>.
pub const ORT_DYLIB_ENV: &str = "ORT_DYLIB_PATH";

/// The ONNX-runtime dylib path to provision via `ORT_DYLIB_PATH`, IFF it exists
/// on disk — else `None`.
///
/// Pure (no env mutation): `std::env::set_var` is `unsafe` in edition 2024 and
/// `unsafe` is forbidden workspace-wide, so the env write happens at the single
/// process-spawn seam (the daemon `LaunchAgent` generator), which this feeds.
///
/// Resolution — fail-open, non-clobbering:
/// 1. An operator-set `ORT_DYLIB_PATH` is authoritative ⇒ `None` (the install
///    seam leaves the existing value untouched).
/// 2. Otherwise, if the conventional absolute path beside the `SharedAI` DB
///    exists on disk ⇒ return it (the generated plist points `ort` at it).
/// 3. Otherwise `None` — never point the env at a non-existent file (that turns
///    a recoverable `ort` search into a hard dlopen failure); let `ort` fall back
///    to its own search (`copy-dylibs` output / system paths).
#[must_use]
pub fn resolve_dylib_path() -> Option<PathBuf> {
    if std::env::var_os(ORT_DYLIB_ENV).is_some() {
        return None; // operator override wins verbatim — never clobber.
    }
    let candidate = conventional_dylib_path();
    candidate.is_file().then_some(candidate)
}

/// The conventional absolute ONNX-runtime path: `<db-parent>/lib/<os-dylib>`.
///
/// Anchored beside `default_db_path()` exactly like the model cache dir, so it is
/// independent of cwd and launch context. Public so the install seam can stage
/// the dylib at the very path the resolver will look for.
#[must_use]
pub fn conventional_dylib_path() -> PathBuf {
    let db = crate::connection::default_db_path();
    // default_db_path is an absolute, multi-segment path, so parent is absolute;
    // the parentless arm is unreachable but still yields an absolute base.
    let base = db.parent().unwrap_or(db.as_path());
    base.join("lib").join(os_dylib_name())
}

/// The platform's ONNX Runtime shared-library filename.
///
/// `ort` (load-dynamic) `dlopen`s a plain `libonnxruntime.{dylib|so}` /
/// `onnxruntime.dll`; the install seam stages the versioned binary under this
/// stable name.
#[must_use]
pub const fn os_dylib_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "libonnxruntime.dylib"
    } else if cfg!(target_os = "windows") {
        "onnxruntime.dll"
    } else {
        "libonnxruntime.so"
    }
}

#[cfg(test)]
#[path = "dylib_path_test.rs"]
mod dylib_path_test;

// Proves the generated LaunchAgent plist bakes ORT_DYLIB_PATH when a runtime is
// resolved, omits the env dict (fail-open) when absent, and escapes XML so a path
// with `&`/`<` cannot break the plist or inject markup.
use super::{render_plist, xml_escape};

#[test]
fn plist_bakes_ort_dylib_path_when_present() {
    let plist = render_plist("/usr/local/bin/kavach", Some("/abs/lib/libonnxruntime.dylib"));
    assert!(plist.contains("<key>EnvironmentVariables</key>"), "env dict missing: {plist}");
    assert!(plist.contains("ORT_DYLIB_PATH"), "env key missing: {plist}");
    assert!(
        plist.contains("<string>/abs/lib/libonnxruntime.dylib</string>"),
        "dylib path not baked: {plist}"
    );
    // The daemon contract: rpc --transport http, KeepAlive, RunAtLoad.
    assert!(plist.contains("<string>rpc</string>"));
    assert!(plist.contains("<key>KeepAlive</key>"));
    assert!(plist.contains("ai.shared.kavach-rpc"));
}

#[test]
fn plist_omits_env_dict_when_no_dylib() {
    // Fail-open: no staged runtime ⇒ no EnvironmentVariables block, so ort falls
    // back to its own search rather than dlopen-ing a path we know is empty.
    let plist = render_plist("/usr/local/bin/kavach", None);
    assert!(
        !plist.contains("EnvironmentVariables"),
        "env dict must be omitted when no dylib resolved: {plist}"
    );
    assert!(!plist.contains("ORT_DYLIB_PATH"));
    // The rest of the plist is still well-formed.
    assert!(plist.contains("</plist>"));
}

#[test]
fn xml_escape_neutralizes_markup_in_paths() {
    // A path with `&`/`<`/`>` must not break the plist XML or inject elements.
    assert_eq!(xml_escape("/a&b/<x>"), "/a&amp;b/&lt;x&gt;");
    let plist = render_plist("/bin/k&v<x>", Some("/lib/a&b.dylib"));
    assert!(plist.contains("/bin/k&amp;v&lt;x&gt;"), "binary path not escaped: {plist}");
    assert!(plist.contains("/lib/a&amp;b.dylib"), "dylib path not escaped: {plist}");
    assert!(!plist.contains("k&v<x>"), "raw unescaped markup leaked: {plist}");
}

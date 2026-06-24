use super::advise;

#[test]
fn new_pub_fn_fires_the_ladder() {
    let old = "pub fn a() {}\n";
    let new = "pub fn a() {}\npub fn parse_config() {}\n";
    let out = advise("crates/x/src/cfg.rs", old, new);
    assert!(out.is_some());
    let m = out.unwrap();
    assert!(m.contains("[REUSE_LADDER]"));
    assert!(m.contains("parse_config"));
    assert!(m.contains("reuse") || m.contains("exist"));
}

#[test]
fn new_pub_struct_and_enum_fire() {
    let old = "";
    let new = "pub struct Widget;\npub enum Mode { A }\n";
    let out = advise("crates/x/src/w.rs", old, new).unwrap();
    assert!(out.contains("Widget"));
    assert!(out.contains("Mode"));
}

#[test]
fn no_new_symbol_is_clean() {
    let old = "pub fn a() {}\npub fn b() {}\n";
    let new = "pub fn a() {}\npub fn b() {} // tweak body\n";
    assert!(advise("crates/x/src/y.rs", old, new).is_none());
}

#[test]
fn private_fn_does_not_fire() {
    let old = "";
    let new = "fn helper() {}\n";
    assert!(advise("crates/x/src/y.rs", old, new).is_none());
}

#[test]
fn test_file_is_exempt() {
    let new = "pub fn brand_new() {}\n";
    assert!(advise("crates/x/src/y_test.rs", "", new).is_none());
}

#[test]
fn non_rust_is_exempt() {
    assert!(advise("README.md", "", "pub fn x() {}\n").is_none());
}

#[test]
fn pre_existing_pub_symbol_not_reflagged() {
    // Editing a file that already had the pub symbol (e.g. moved within the file)
    // must not re-fire — only a symbol absent from `old` counts as new.
    let old = "pub fn keep() {}\npub fn other() {}\n";
    let new = "pub fn other() {}\npub fn keep() {}\n";
    assert!(advise("crates/x/src/y.rs", old, new).is_none());
}

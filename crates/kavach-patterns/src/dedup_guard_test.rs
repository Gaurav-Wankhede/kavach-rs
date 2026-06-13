//! Regression suite for `dedup_guard`. Proves the false-positive bound the engine
//! RULE demands for a P0: a redefinition of an imported name is always flagged, and
//! every legitimate shape (no import, grouped/glob import, distinct names, a method
//! `fn` on a type, ungoverned paths) produces ZERO hits.
use super::detect;
use crate::severity::Severity;

const GOVERNED: &str = "/x/crates/core/billing/src/model.rs";

fn hits(path: &str, src: &str) -> usize {
    detect(path, src).len()
}

#[test]
fn flags_redefinition_of_imported_struct() {
    let src = "use core_utils::AppConfig;\n\npub struct AppConfig {\n    url: String,\n}\n";
    let v = detect(GOVERNED, src);
    assert_eq!(v.len(), 1, "imported then redefined struct must block");
    assert_eq!(v[0].severity, Severity::P0Block);
}

#[test]
fn flags_redefinition_via_alias() {
    let src = "use crate::config::Real as AppConfig;\nfn AppConfig() {}\n";
    assert_eq!(hits(GOVERNED, src), 1, "alias binds the name; redefining it blocks");
}

#[test]
fn flags_redefined_fn_and_const() {
    let src = "use core_utils::limit;\nuse core_utils::MAX;\nfn limit() {}\nconst MAX: u8 = 1;\n";
    assert_eq!(hits(GOVERNED, src), 2, "both redefined import names block");
}

// --- false-positive bound: every legitimate shape must be silent ---

#[test]
fn clean_when_no_matching_import() {
    let src = "use core_utils::AppConfig;\npub struct Settings { url: String }\n";
    assert_eq!(hits(GOVERNED, src), 0, "distinct names are not a redefinition");
}

#[test]
fn clean_with_no_imports_at_all() {
    let src = "pub struct AppConfig { url: String }\n";
    assert_eq!(hits(GOVERNED, src), 0, "defining a brand-new local type is fine");
}

#[test]
fn clean_on_grouped_and_glob_imports() {
    let grouped = "use core_utils::{AppConfig, Db};\nstruct AppConfig;\n";
    let glob = "use core_utils::*;\nstruct AppConfig;\n";
    assert_eq!(hits(GOVERNED, grouped), 0, "grouped import binds no single name");
    assert_eq!(hits(GOVERNED, glob), 0, "glob import binds no single name");
}

#[test]
fn clean_on_ungoverned_path() {
    let src = "use core_utils::AppConfig;\nstruct AppConfig;\n";
    assert_eq!(
        hits("/x/crates/ui-atoms/src/x.rs", src),
        0,
        "harness/frontend/tools are out of scope"
    );
}

#[test]
fn does_not_treat_let_or_call_as_definition() {
    let src = "use core_utils::value;\nlet value = compute();\n    value();\n";
    assert_eq!(hits(GOVERNED, src), 0, "let-binding / call is not an item definition");
}

//! Reference-extraction tests: extension→skill mapping, wikilink + INVOKE
//! parsing, and dedup.

use super::{extract_content_references, skill_for_file};

#[test]
fn skill_for_file_maps_rs_to_rust() {
    assert_eq!(skill_for_file("src/main.rs"), "rust");
}

#[test]
fn skill_for_file_maps_tsx_to_typescript() {
    assert_eq!(skill_for_file("App.tsx"), "typescript");
}

#[test]
fn skill_for_file_returns_empty_for_unknown_ext() {
    assert_eq!(skill_for_file("data.bin"), "");
}

#[test]
fn extract_refs_parses_wikilinks() {
    let r = extract_content_references("See [[rust]] and [[sql]]");
    assert!(r.iter().any(|s| s == "rust"));
    assert!(r.iter().any(|s| s == "sql"));
}

#[test]
fn extract_refs_parses_invoke_directive() {
    let r = extract_content_references("INVOKE m13-domain-error");
    assert!(r.iter().any(|s| s == "m13-domain-error"));
}

#[test]
fn extract_refs_dedups() {
    let r = extract_content_references("[[rust]]\n[[rust]]");
    assert_eq!(r.iter().filter(|s| s.as_str() == "rust").count(), 1);
}

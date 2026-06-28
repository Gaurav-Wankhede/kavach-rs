use super::scan;
use crate::cmd::audit::finding::Lens;

#[test]
fn flags_double_clone() {
    let f = scan("a.rs", "    let x = y.clone().clone();\n");
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].lens, Lens::Yagni);
    assert_eq!(f[0].line, 1);
}

#[test]
fn flags_dead_code_allow() {
    let f = scan("a.rs", "#[allow(dead_code)]\nfn z() {}\n");
    assert!(f.iter().any(|x| x.hint.contains("dead-code")));
}

#[test]
fn clean_source_is_empty() {
    assert!(scan("a.rs", "fn ok() { let v = 1; }\n").is_empty());
}

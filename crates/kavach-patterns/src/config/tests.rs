use super::*;

#[test]
fn test_defaults() {
    let g = load();
    assert!(!g.as_ref().unwrap().blocked.is_empty());
    drop(g);
}

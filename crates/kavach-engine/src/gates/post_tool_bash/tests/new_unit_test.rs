//! Tests for a brand-new unit (this file tests `new_unit` module)
#[test]
fn test_new_unit_basic() {
    let x = super::super::new_unit::greet("Alice");
    assert_eq!(x, "Hello, Alice!");
}

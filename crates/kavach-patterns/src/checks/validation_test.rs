use super::*;

#[test]
fn test_ident() {
    assert!(validate_identifier("foo-1").is_ok());
    assert!(validate_identifier("").is_err());
}

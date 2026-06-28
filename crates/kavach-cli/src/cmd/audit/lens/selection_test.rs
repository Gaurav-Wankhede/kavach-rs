use super::Selection;

#[test]
fn maps_known_lenses() {
    assert_eq!(Selection::from_flag("code"), Selection::Code);
    assert_eq!(Selection::from_flag("self"), Selection::SelfAudit);
    assert_eq!(Selection::from_flag("security"), Selection::Security);
    assert_eq!(Selection::from_flag("all"), Selection::All);
}

#[test]
fn unknown_defaults_to_all() {
    assert_eq!(Selection::from_flag("garbage"), Selection::All);
    assert_eq!(Selection::from_flag(""), Selection::All);
}

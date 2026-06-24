use super::marker_note;

#[test]
fn intentional_marker_with_upgrade_trigger_is_clean() {
    let (note, has_trigger) =
        marker_note("// kavach:intentional cohesive hub; upgrade when it exceeds 3 concerns").unwrap();
    assert!(note.contains("cohesive hub"));
    assert!(has_trigger);
}

#[test]
fn marker_without_trigger_flags() {
    let (_note, has_trigger) = marker_note("// kavach:intentional hardcoded list for now").unwrap();
    assert!(!has_trigger);
}

#[test]
fn non_marker_line_is_none() {
    assert!(marker_note("    let x = 42;").is_none());
}

#[test]
fn when_clause_counts_as_trigger() {
    let (_n, has_trigger) = marker_note("// ponytail: single impl; split when a 2nd caller appears").unwrap();
    assert!(has_trigger);
}

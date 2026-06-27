use super::check_tool_search;

#[test]
fn advises_on_symbol_grep() {
    assert!(check_tool_search("Grep", "RoleQuery").is_some());
}

#[test]
fn advises_on_glob_symbol() {
    assert!(check_tool_search("Glob", "build_context").is_some());
}

#[test]
fn ignores_free_text() {
    assert!(check_tool_search("Grep", "fn foo(").is_none());
}

#[test]
fn ignores_regex_pattern() {
    assert!(check_tool_search("Grep", "TODO|FIXME").is_none());
}

#[test]
fn ignores_glob_star() {
    assert!(check_tool_search("Glob", "**/*.rs").is_none());
}

#[test]
fn ignores_other_tools() {
    assert!(check_tool_search("Read", "RoleQuery").is_none());
}

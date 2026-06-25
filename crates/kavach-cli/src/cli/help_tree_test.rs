use super::render;

#[test]
fn tree_starts_at_kavach_root() {
    assert!(render().starts_with("kavach\n"));
}

#[test]
fn tree_lists_db_and_its_nested_actions() {
    let t = render();
    assert!(t.contains("db"), "top-level db missing");
    assert!(t.contains("kanban"), "nested db action missing");
}

#[test]
fn tree_omits_the_help_pseudo_command() {
    assert!(!render().lines().any(|l| l.trim() == "help"));
}

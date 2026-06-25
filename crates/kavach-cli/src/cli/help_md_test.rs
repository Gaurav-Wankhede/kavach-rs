use super::render;

#[test]
fn reference_has_root_and_per_command_sections() {
    let md = render();
    assert!(md.starts_with("# kavach CLI reference"));
    assert!(md.contains("## `kavach db`"));
    assert!(md.contains("## `kavach db kanban`"));
}

#[test]
fn committed_cli_md_is_in_sync() {
    let committed = include_str!("../../../../docs/CLI.md");
    let generated = format!("{}\n", render().trim_end());
    assert_eq!(
        generated, committed,
        "docs/CLI.md is stale — regenerate: kavach commands --markdown > docs/CLI.md"
    );
}

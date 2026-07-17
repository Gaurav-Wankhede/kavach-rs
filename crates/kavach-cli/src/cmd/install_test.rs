use super::*;

#[test]
fn resolve_all_expands_to_shipped() {
    assert_eq!(resolve("all").unwrap().len(), Target::all().len());
}

#[test]
fn resolve_single_known() {
    assert_eq!(resolve("cursor").unwrap(), vec![Target::Cursor]);
    assert_eq!(resolve("CC").unwrap(), vec![Target::ClaudeCode]);
}

#[test]
fn resolve_unknown_is_error() {
    assert!(resolve("bogus").is_err());
}

#[test]
fn pi_now_ships_and_dry_run_succeeds() {
    // Pi's TS extension template now ships, so install_one resolves a template
    // and the dry-run reports a DryRun outcome (no longer the unshipped error).
    let bin = std::path::Path::new("/x/kavach");
    let line = install_one(Target::Pi, bin, true).unwrap();
    assert!(line.contains("[pi]"), "{line}");
    assert!(
        line.contains("index.ts"),
        "Pi installs to the extension path: {line}"
    );
}

#[test]
fn kimi_dry_run_reports_both_hooks_and_directives() {
    let bin = std::path::Path::new("/x/kavach");
    let line = install_one(Target::Kimi, bin, true).unwrap();
    assert!(line.contains("[kimi]"), "{line}");
    assert!(line.contains("config.toml"), "{line}");
    assert!(line.contains("AGENTS.md"), "{line}");
}

#[test]
fn directives_path_per_vendor() {
    assert_eq!(
        Target::ClaudeCode.rel_directives_path(),
        Some(".claude/CLAUDE.md")
    );
    assert_eq!(
        Target::Cursor.rel_directives_path(),
        Some(".cursor/rules/kavach.mdc")
    );
    assert_eq!(
        Target::Codex.rel_directives_path(),
        Some(".codex/AGENTS.md")
    );
    assert_eq!(Target::Antigravity.rel_directives_path(), None);
    assert_eq!(Target::Pi.rel_directives_path(), None);
    assert_eq!(
        Target::Kimi.rel_directives_path(),
        Some(".kimi-code/AGENTS.md")
    );
}

#[test]
fn directives_template_matches_path_presence() {
    for t in Target::all() {
        assert_eq!(
            t.rel_directives_path().is_some(),
            t.directives_template().is_some(),
            "{}: path/template Option must agree",
            t.name()
        );
    }
}

#[test]
fn cc_dry_run_reports_both_hooks_and_directives() {
    let bin = std::path::Path::new("/x/kavach");
    let line = install_one(Target::ClaudeCode, bin, true).unwrap();
    assert!(line.contains("settings.json"), "{line}");
    assert!(line.contains("CLAUDE.md"), "{line}");
}

use super::check_toolbelt_cli;

#[test]
fn fires_on_jq() {
    let a = check_toolbelt_cli("kavach context | jq '.kanban'").expect("jq must advise");
    assert!(a.contains("[ADVISORY:toolbelt]"));
    assert!(a.contains("`jq` → `jaq`"));
}

#[test]
fn fires_on_leading_sed() {
    let a = check_toolbelt_cli("sed -i 's/a/b/' f.txt").expect("sed must advise");
    assert!(a.contains("`sed` → `sd`"));
}

#[test]
fn fires_on_multiple() {
    let a = check_toolbelt_cli("find . -name '*.rs' | curl -X POST").expect("multi must advise");
    assert!(a.contains("`find` → `fd`"));
    assert!(a.contains("`curl` → `xh`"));
}

#[test]
fn silent_on_substring_not_command_word() {
    // `oldfind`, `category`, `procstat` are not the legacy tool.
    assert!(check_toolbelt_cli("oldfind --help").is_none());
    assert!(check_toolbelt_cli("echo category > f").is_none());
}

#[test]
fn silent_on_quoted_mention() {
    // The tool name inside a string is data, not a call.
    assert!(check_toolbelt_cli("echo 'use jq for json'").is_none());
}

#[test]
fn silent_on_clean_command() {
    assert!(check_toolbelt_cli("cargo nextest run").is_none());
    assert!(check_toolbelt_cli("rg pattern src/").is_none());
}

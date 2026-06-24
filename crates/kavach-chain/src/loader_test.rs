//! `scan_all_agents` underscore-skip: a `_*.md` file is a shared prompt fragment
//! (e.g. `_scope-guard.md`), NOT a routable agent. The scan must not cache it.
use super::DynamicLoader;
use std::fs;
use std::path::PathBuf;

/// Build a loader over a fresh temp agent_dir holding one real agent + one
/// underscore fragment, scan it, and return (loader, dir) for assertions.
fn scan_dir_with(real: &str, frag: &str) -> (DynamicLoader, PathBuf) {
    // Unique temp dir keyed by the test's distinct (real, frag) names — no
    // Date/random available in this crate's test env, so vary by content.
    let base = std::env::temp_dir().join(format!("kavach-loader-{real}-{frag}"));
    drop(fs::remove_dir_all(&base));
    fs::create_dir_all(&base).expect("mk temp agent_dir");
    fs::write(
        base.join(format!("{real}.md")),
        "---\nname: realone\ndescription: Use this agent to do the real thing.\n---\nbody",
    )
    .expect("write real agent");
    fs::write(
        base.join(format!("{frag}.md")),
        "# Shared fragment — not an agent, no frontmatter\nprompt text",
    )
    .expect("write fragment");
    let skill_dir = base.join("skills");
    let loader = DynamicLoader::new(base.clone(), skill_dir);
    drop(loader.scan_all_agents());
    (loader, base)
}

#[test]
fn scan_skips_underscore_prefixed_files() {
    let (loader, dir) = scan_dir_with("realone", "_scope-guard");
    let loaded = loader.loaded_agents();
    assert!(
        loaded.iter().any(|n| n == "realone"),
        "real agent must load: {loaded:?}"
    );
    assert!(
        !loaded.iter().any(|n| n.starts_with('_')),
        "underscore fragment must NOT be cached as an agent: {loaded:?}"
    );
    drop(fs::remove_dir_all(dir));
}

#[test]
fn scan_count_excludes_underscore_fragment() {
    let (loader, dir) = scan_dir_with("solo", "_frag");
    // One real agent + one fragment present; only the real one counts.
    assert_eq!(loader.loaded_agents().len(), 1, "only the real agent is cached");
    drop(fs::remove_dir_all(dir));
}

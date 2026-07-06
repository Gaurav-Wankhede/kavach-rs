use super::*;

#[test]
fn render_pins_absolute_binary() {
    let out = render(
        "kavach gates stop --hook",
        Path::new("/usr/local/bin/kavach"),
    );
    assert_eq!(out, "/usr/local/bin/kavach gates stop --hook");
}

#[test]
fn create_then_idempotent_rerun() {
    let dir = std::env::temp_dir().join(format!("kvinst-{}", std::process::id()));
    let p = dir.join("cfg.json");
    fs::remove_dir_all(&dir).ok();
    assert_eq!(install(&p, "A", false).unwrap(), Outcome::Created);
    // Re-run with identical body: no-op, no backup spawned.
    assert_eq!(install(&p, "A", false).unwrap(), Outcome::Unchanged);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn overwrite_backs_up_prior() {
    let dir = std::env::temp_dir().join(format!("kvinst-ow-{}", std::process::id()));
    let p = dir.join("cfg.json");
    fs::remove_dir_all(&dir).ok();
    install(&p, "OLD", false).unwrap();
    assert_eq!(install(&p, "NEW", false).unwrap(), Outcome::Overwrote);
    let bak = fs::read_to_string(dir.join("cfg.json.kavach.bak")).unwrap();
    assert_eq!(bak, "OLD", "prior content must survive in the backup");
    assert_eq!(fs::read_to_string(&p).unwrap(), "NEW");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn dry_run_writes_nothing() {
    let dir = std::env::temp_dir().join(format!("kvinst-dry-{}", std::process::id()));
    let p = dir.join("cfg.json");
    fs::remove_dir_all(&dir).ok();
    assert!(matches!(install(&p, "X", true).unwrap(), Outcome::DryRun(_)));
    assert!(!p.exists(), "dry-run must not create the file");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn directives_absent_writes_fresh() {
    let dir = std::env::temp_dir().join(format!("kvdir-abs-{}", std::process::id()));
    let p = dir.join("CLAUDE.md");
    fs::remove_dir_all(&dir).ok();
    let msg = install_directives_if_absent(&p, "BODY", false).unwrap();
    assert!(msg.contains("created"), "{msg}");
    assert_eq!(fs::read_to_string(&p).unwrap(), "BODY");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn directives_present_backs_up_and_skips() {
    let dir = std::env::temp_dir().join(format!("kvdir-pres-{}", std::process::id()));
    let p = dir.join("CLAUDE.md");
    fs::remove_dir_all(&dir).ok();
    fs::create_dir_all(&dir).unwrap();
    fs::write(&p, "USER WRITTEN").unwrap();
    let msg = install_directives_if_absent(&p, "NEW BODY", false).unwrap();
    assert!(msg.contains("kept"), "{msg}");
    assert_eq!(fs::read_to_string(&p).unwrap(), "USER WRITTEN", "never clobbered");
    let bak = fs::read_to_string(dir.join("CLAUDE.md.kavach.bak")).unwrap();
    assert_eq!(bak, "USER WRITTEN");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn directives_dry_run_writes_nothing() {
    let dir = std::env::temp_dir().join(format!("kvdir-dry-{}", std::process::id()));
    let p = dir.join("CLAUDE.md");
    fs::remove_dir_all(&dir).ok();
    let msg = install_directives_if_absent(&p, "BODY", true).unwrap();
    assert!(msg.contains("would"), "{msg}");
    assert!(!p.exists());
    fs::remove_dir_all(&dir).ok();
}

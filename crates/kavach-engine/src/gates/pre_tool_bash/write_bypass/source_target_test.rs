//! Proofs for `targets_tracked_source`: the deny must fire for a Bash write to a
//! Rust SOURCE file (the capability-laundering case) and must NOT fire for a
//! generated artifact (the benign advisory case). The false-positive bound is
//! what justifies promoting this branch to a P0 deny per the severity policy.

use super::targets_tracked_source;

#[test]
fn the_python_heredoc_bypass_that_started_this_is_caught() {
    // The exact shape used to launder an Edit on write.rs past the gate.
    let cmd = "python3 - <<'PY'\nopen('crates/kavach-surreal/src/write.rs','w').write(s)\nPY";
    assert!(targets_tracked_source(cmd), "heredoc write to a .rs source must be denied");
}

#[test]
fn redirect_into_a_rust_source_is_caught() {
    assert!(targets_tracked_source("echo x > crates/foo/src/lib.rs"));
    assert!(targets_tracked_source("cat t >> src/main.rs"));
}

#[test]
fn sed_inplace_on_a_source_path_is_caught() {
    assert!(targets_tracked_source("sed -i 's/a/b/' crates/kavach-engine/src/gates/mod.rs"));
}

#[test]
fn tests_tree_and_other_source_langs_are_caught() {
    assert!(targets_tracked_source("python3 -c \"open('tests/foo.rs','w')\""));
    assert!(targets_tracked_source("echo x > src/app.ts"));
    assert!(targets_tracked_source("echo q > crates/db/src/migrate.sql"));
}

#[test]
fn generated_artifacts_are_not_denied() {
    // These remain the benign ADVISORY case — a Bash write here is normal.
    assert!(!targets_tracked_source("echo x > .config/nextest.toml"));
    assert!(!targets_tracked_source("python3 emit.py > loop.yaml"));
    assert!(!targets_tracked_source("echo x > Cargo.toml"));
    assert!(!targets_tracked_source("cat > /tmp/scratch.json"));
    assert!(!targets_tracked_source("echo x > README.md"));
}

#[test]
fn a_source_extension_outside_a_source_tree_is_not_denied() {
    // A `.rs` written to /tmp is not a tracked source edit — needs BOTH signals.
    assert!(!targets_tracked_source("echo x > /tmp/throwaway.rs"));
    // A source tree without a source extension (a data file) is also fine.
    assert!(!targets_tracked_source("echo x > crates/foo/data.json"));
}

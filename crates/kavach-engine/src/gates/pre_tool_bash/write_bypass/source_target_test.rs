//! Proofs for `targets_tracked_source`: the deny must fire for a Bash write to a
//! Rust SOURCE file (the capability-laundering case) and must NOT fire for a
//! generated artifact (the benign advisory case). The false-positive bound is
//! what justifies promoting this branch to a P0 deny per the severity policy.

use super::targets_tracked_source;

#[test]
fn the_python_heredoc_bypass_that_started_this_is_caught() {
    // The exact shape used to launder an Edit on write.rs past the gate.
    let cmd = "python3 - <<'PY'\nopen('crates/kavach-surreal/src/write.rs','w').write(s)\nPY";
    assert!(
        targets_tracked_source(cmd),
        "heredoc write to a .rs source must be denied"
    );
}

#[test]
fn redirect_into_a_rust_source_is_caught() {
    assert!(targets_tracked_source("echo x > crates/foo/src/lib.rs"));
    assert!(targets_tracked_source("cat t >> src/main.rs"));
}

#[test]
fn sed_inplace_on_a_source_path_is_caught() {
    assert!(targets_tracked_source(
        "sed -i 's/a/b/' crates/kavach-engine/src/gates/mod.rs"
    ));
}

#[test]
fn tests_tree_and_other_source_langs_are_caught() {
    assert!(targets_tracked_source(
        "python3 -c \"open('tests/foo.rs','w')\""
    ));
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
fn a_kavach_db_write_carrying_a_source_path_in_content_is_not_denied() {
    // REGRESSION: `kavach db write --content "...crates/x/src/y.rs..."` is an RPC,
    // not a file write — the source path is prose in the --content arg. The gate
    // must not deny it (this exact shape false-positived after the deny shipped).
    let cmd = "kavach db write --new --project p --category decision --key k \
               --title t --content \"shipped crates/kavach-rpc/src/methods/db/ope.rs\"";
    assert!(
        !targets_tracked_source(cmd),
        "a kavach db write is not a source mutation"
    );
    // Also the chained form actually used (cd then kavach db).
    let chained = "kavach db write --content \"see crates/foo/src/lib.rs for detail\"";
    assert!(!targets_tracked_source(chained));
}

#[test]
fn a_source_extension_outside_a_source_tree_is_not_denied() {
    // A `.rs` written to /tmp is not a tracked source edit — needs BOTH signals.
    assert!(!targets_tracked_source("echo x > /tmp/throwaway.rs"));
    // A source tree without a source extension (a data file) is also fine.
    assert!(!targets_tracked_source("echo x > crates/foo/data.json"));
}

#[test]
fn an_absolute_path_in_another_project_is_not_denied() {
    // REGRESSION (the false-positive this fix targets): a Bash write to a SOURCE
    // file in a DIFFERENT project — an absolute path NOT under this workspace —
    // is outside the pre-write gate's jurisdiction, so it must PASS. Before the
    // jurisdiction check, the `/src/` + `.ts` substrings alone tripped the deny
    // and blocked all cross-project work.
    let foreign_cp = "cp /tmp/poc-spec.ts \
                      /Users/gauravwankhede/Projects/Video/kavach-ad/src/poc-spec.ts";
    assert!(
        !targets_tracked_source(foreign_cp),
        "a write into another project's src/ tree is out of jurisdiction"
    );
    let foreign_heredoc = "cat > /Users/gauravwankhede/Projects/Video/kavach-ad/src/index.tsx";
    assert!(
        !targets_tracked_source(foreign_heredoc),
        "a heredoc write into a foreign project's src/ is out of jurisdiction"
    );
    // An absolute /tmp source file is likewise external.
    assert!(!targets_tracked_source(
        "cp /tmp/a.rs /var/folders/scratch/b.rs"
    ));
}

#[test]
fn an_absolute_path_inside_this_workspace_is_still_denied() {
    // The launder protection MUST survive for THIS repo: an absolute path that
    // resolves under the workspace root is in-jurisdiction and stays a DENY.
    // Build it from the real workspace root so the test is location-independent.
    let root = std::env::current_dir()
        .ok()
        .and_then(|cwd| {
            cwd.ancestors()
                .find(|d| d.join("Cargo.toml").is_file())
                .map(std::path::Path::to_path_buf)
        })
        .expect("tests run inside the kavach-rs workspace");
    let in_repo = format!(
        "cp /tmp/x.rs {}/crates/kavach-surreal/src/write.rs",
        root.display()
    );
    assert!(
        targets_tracked_source(&in_repo),
        "an absolute path under the workspace root is in-jurisdiction → deny"
    );
}

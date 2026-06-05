//! §RADIUS-INTEGRITY contract tests: the CWE-184 quote-strip trade-off + the
//! shell/symlink preserve-payload coverage. Both halves MUST hold.
use crate::gates::prod_guard::destructive::check_prod_destructive;

#[test]
fn non_db_tool_quoted_destructive_keywords_not_blocked() {
    // The tmp-txt-workaround root: non-DB tools whose quoted args MERELY
    // MENTION destructive verbs as data must NOT trip the guard.
    assert!(
        check_prod_destructive("git commit -m \"fix: prevent DROP DATABASE foo from running\"")
            .is_none(),
        "git commit message mentioning DROP DATABASE must not block"
    );
    assert!(
        check_prod_destructive("kavach db write --content \"explain TRUNCATE risk for prod\"")
            .is_none(),
        "kavach db --content mentioning TRUNCATE must not block"
    );
    assert!(check_prod_destructive("echo 'do not run drop database here'").is_none());
    assert!(
        check_prod_destructive("rg -n 'truncate ' migrations/").is_none(),
        "rg pattern containing TRUNCATE must not block"
    );
}

#[test]
fn db_client_dash_c_destructive_still_blocked() {
    // Pocket-OS vector: psql/mysql with -c/-e delivering a destructive payload
    // as a quoted arg MUST still HARD-BLOCK (no quote-strip for DB clients).
    assert!(
        check_prod_destructive("psql -h prod.example.com -c 'DROP DATABASE foo'").is_some(),
        "psql -c with DROP DATABASE on non-local host must HARD-BLOCK"
    );
    assert!(
        check_prod_destructive("mysql -h prod -e \"DROP DATABASE app\"").is_some(),
        "mysql -e with DROP DATABASE must HARD-BLOCK"
    );
    assert!(
        check_prod_destructive("psql prod -c 'TRUNCATE users'").is_some(),
        "psql -c with TRUNCATE must HARD-BLOCK"
    );
    assert!(
        check_prod_destructive("echo 'DROP DATABASE foo' | psql prod").is_some(),
        "echo piped into psql with DROP DATABASE must HARD-BLOCK"
    );
    assert!(
        check_prod_destructive("psql -h localhost -c 'DROP DATABASE test_db'").is_none(),
        "localhost test_db destruction is the existing exemption"
    );
}

#[test]
fn nested_shell_dash_c_with_destructive_still_blocked() {
    // Reviewer-discovered P1 bypass: shells (bash/sh/zsh/...) are in the same
    // preserve-payload class as DB clients (their `-c` arg is a verbatim command).
    assert!(
        check_prod_destructive("bash -c 'DROP DATABASE foo'").is_some(),
        "bash -c 'DROP DATABASE foo' must HARD-BLOCK"
    );
    assert!(
        check_prod_destructive("sh -c \"DROP DATABASE foo\"").is_some(),
        "sh -c with destructive payload must HARD-BLOCK"
    );
    assert!(
        check_prod_destructive("zsh -c 'TRUNCATE users'").is_some(),
        "zsh -c with destructive payload must HARD-BLOCK"
    );
    assert!(
        check_prod_destructive("bash -c \"echo 'DROP DATABASE foo' | psql prod\"").is_some(),
        "bash -c that pipes into psql must HARD-BLOCK"
    );
}

#[test]
fn symlink_with_canonical_name_still_blocked() {
    // A path whose BASENAME matches a known DB client is classified by
    // rsplit('/') on the command-position word → preserves -c payload.
    assert!(
        check_prod_destructive("/tmp/psql -h prod -c 'DROP DATABASE foo'").is_some(),
        "absolute path to a known DB-client name must HARD-BLOCK"
    );
    assert!(
        check_prod_destructive("./psql -h prod -c 'DROP DATABASE foo'").is_some(),
        "relative path to a known DB-client name must HARD-BLOCK"
    );
}

#[test]
#[ignore = "KNOWN LOW-risk boundary: a symlink with a RENAMED basename \
    (ln -s /usr/bin/psql /tmp/innocent) bypasses is_db_client_command \
    because the basename is not in DB_CLIENTS. Closing it needs gate-time \
    symlink resolution, which the gate intentionally does NOT do. Re-enable \
    when a syscall-based canonicalization layer is added."]
fn symlink_with_renamed_basename_is_a_known_gap() {
    assert!(
        check_prod_destructive("/tmp/innocent -h prod -c 'DROP DATABASE foo'").is_some(),
        "renamed-symlink bypass — KNOWN gap, see test attribute"
    );
}

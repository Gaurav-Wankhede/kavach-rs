use super::check_secret_cli_read;

#[test]
fn fires_on_bare_provider_read() {
    // Bare read prints to stdout → value enters context → advisory.
    let a = check_secret_cli_read("wrangler secret list").expect("bare read must fire");
    assert!(a.contains("[ADVISORY:secret-read]"));
    assert!(a.contains("/tmp/op.sh"));
}

#[test]
fn fires_on_piped_reveal_sink() {
    let a = check_secret_cli_read("vault kv get -field=token secret/app | cat")
        .expect("piped-to-cat read must fire");
    assert!(a.contains("[ADVISORY:secret-read]"));
}

#[test]
fn silent_when_redirected_to_file_no_reader() {
    // Redirected to a file with no reader sink — value not surfaced to context.
    assert!(
        check_secret_cli_read("aws secretsmanager get-secret-value --secret-id x > /tmp/s")
            .is_none()
    );
}

#[test]
fn silent_on_write_verb() {
    // `put`/`set` is the safe WRITE op — must not fire.
    assert!(check_secret_cli_read("wrangler secret put ALLOWED_ORIGINS").is_none());
}

#[test]
fn silent_on_quoted_mention() {
    // The verb inside a quoted string is data, not a call.
    assert!(check_secret_cli_read("echo 'run wrangler secret list to debug'").is_none());
}

#[test]
fn silent_on_unrelated_command() {
    assert!(check_secret_cli_read("cargo nextest run").is_none());
}

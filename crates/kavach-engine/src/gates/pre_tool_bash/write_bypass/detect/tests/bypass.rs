//! `is_write_bypass`: redirect, tee, sed, and Python file/DB-write cases.

use super::super::is_write_bypass;

#[test]
fn test_write_bypass_blocked() {
    assert!(is_write_bypass("sed -i 's/old/new/' file.rs"));
    assert!(is_write_bypass("echo 'data' > output.txt"));
    assert!(is_write_bypass("cat template.txt >> config.toml"));
    assert!(is_write_bypass("cmd | tee /path/to/file"));
    assert!(is_write_bypass("python3 -c \"open('f','w').write('x')\""));
    assert!(is_write_bypass(
        "python3 -c \"open('out.txt','wb').write(b'x')\""
    ));
}

#[test]
fn test_python_pipe_to_psql_blocked() {
    assert!(is_write_bypass(
        "python3 -c \"print('SELECT 1')\" | psql $DATABASE_URL"
    ));
    assert!(is_write_bypass(
        "python3 -c \"import hashlib; print(sql)\" |psql $URL"
    ));
}

#[test]
fn test_write_bypass_allowed() {
    assert!(!is_write_bypass("cargo test 2>&1"));
    assert!(!is_write_bypass("echo hello"));
    assert!(!is_write_bypass("cargo check 2>&1 | tail -5"));
    assert!(!is_write_bypass("cat file.rs"));
    assert!(!is_write_bypass("grep pattern file.rs"));
    assert!(!is_write_bypass("kavach db query > /tmp/kavach_out.txt"));
    assert!(!is_write_bypass("grep -i 'sedan' file.txt"));
    assert!(!is_write_bypass("grep -i sediment log.txt"));
}

#[test]
fn quoted_redirect_glyph_in_arg_not_a_bypass() {
    // FP fixed: a `>` (or `<placeholder>`) INSIDE a quoted argument is data, not
    // a shell redirect. `kavach db write --content "...<cmd>... a>b ..."` writes a
    // DB row through the single-writer daemon, not a file — must NOT be blocked.
    assert!(!is_write_bypass(
        "kavach db write --new --project p --category decision --key k --title t --content \"use set -a && . ./.env && set +a && <cmd>; CREATE TABLE IF NOT EXISTS x; widens 3 -> 4\""
    ));
    assert!(!is_write_bypass(
        "kavach db write --content 'derive <canonical def> then run <bin>'"
    ));
    assert!(!is_write_bypass("echo 'a>b and c>d are data'"));
    assert!(!is_write_bypass("printf '%s' \"len > 5 ? yes : no\""));
}

#[test]
fn multibyte_utf8_in_content_does_not_panic() {
    // Malformed-input lens: a multi-byte char (→, em-dash, emoji) adjacent to a
    // separator must not panic the byte-indexed splitter. Must classify as data.
    assert!(!is_write_bypass(
        "kavach db write --content \"step 3 → 4; widens — see 日本語 and 🚀; CREATE TABLE x\""
    ));
    assert!(!is_write_bypass("echo 'café → résumé; naïve'"));
}

#[test]
fn real_redirect_outside_quotes_still_blocked() {
    // The quote-skip must NOT weaken detection: a genuine redirect to a source
    // file, even after a quoted arg, is still a bypass.
    assert!(is_write_bypass(
        "kavach db get x --content 'note' > /Users/me/proj/src/lib.rs"
    ));
    assert!(is_write_bypass("echo 'safe text' > config.toml"));
    assert!(is_write_bypass("printf '%s' \"$DATA\" >> out.sql"));
}

#[test]
fn python_subprocess_with_open_in_arg_not_blocked() {
    assert!(!is_write_bypass(
        "python3 -c \"import subprocess,os; subprocess.run(['psql', os.environ['DATABASE_URL']], input=sql, capture_output=True)\""
    ));
    assert!(!is_write_bypass(
        "python3 -c \"content = open('file.sql','r').read()\""
    ));
    assert!(!is_write_bypass(
        "python3 -c \"import hashlib; ck = hashlib.sha384(open('f.sql','rb').read()).hexdigest()\""
    ));
}

#[test]
fn test_numeric_comparison_not_redirect() {
    assert!(!is_write_bypass(
        "kavach db write --project x --content \"len > 80 chars\""
    ));
    assert!(!is_write_bypass(
        "kavach db write --project x --content \"count > 0\""
    ));
    assert!(!is_write_bypass(
        "kavach db write --project x --content \"cmd[..80] when len > 80\""
    ));
}

#[test]
fn test_real_redirect_still_blocked() {
    assert!(is_write_bypass("echo hello > output.txt"));
    assert!(is_write_bypass("cat file >> log.txt"));
}

#[test]
fn test_no_space_redirect_blocked() {
    assert!(is_write_bypass("echo data>output.txt"));
    assert!(is_write_bypass("echo data >output.txt"));
    assert!(is_write_bypass("echo data> output.txt"));
    assert!(is_write_bypass("cat tmpl.txt>>config.toml"));
    assert!(is_write_bypass("printf 'x'>src/lib.rs"));
    assert!(is_write_bypass("true; echo y>z.rs"));
}

#[test]
fn test_no_space_redirect_safe_targets_still_allowed() {
    assert!(!is_write_bypass("test 2>/dev/null"));
    assert!(!is_write_bypass(
        "kavach db write --project x --content \"len>80\""
    ));
    assert!(!is_write_bypass(
        "kavach db write --project x --content \"count>0 always\""
    ));
    assert!(!is_write_bypass("cargo test 2>&1 | tail -5"));
    assert!(!is_write_bypass("kavach db query>/tmp/kavach_o.txt"));
    assert!(!is_write_bypass("echo foo->bar baz"));
}

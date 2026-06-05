//! `is_write_bypass`: file-writing tool detection (wget/curl/cp/mv/install/dd),
//! safe-sink + read-only exemptions, and the §DEPLOY kavach-binary exemption.

use super::super::is_write_bypass;

#[test]
fn test_tool_file_write_blocked() {
    assert!(is_write_bypass("wget -O /tmp/x https://e/x"));
    assert!(is_write_bypass(
        "wget --output-document=/etc/cfg https://e/x"
    ));
    assert!(is_write_bypass("curl -o src/lib.rs https://e/x"));
    assert!(is_write_bypass("curl --output src/lib.rs https://e/x"));
    assert!(is_write_bypass("curl -O https://e/payload.rs"));
    assert!(is_write_bypass("cp /tmp/payload.rs crates/foo/src/lib.rs"));
    assert!(is_write_bypass("mv /tmp/x crates/foo/src/lib.rs"));
    assert!(is_write_bypass(
        "install -m644 /tmp/p crates/foo/src/lib.rs"
    ));
    assert!(is_write_bypass("dd if=/dev/zero of=crates/foo/src/lib.rs"));
    assert!(is_write_bypass("true && curl -o x.rs https://e/x"));
}

#[test]
fn test_tool_file_write_safe_not_blocked() {
    assert!(!is_write_bypass("curl https://example.com/api"));
    assert!(!is_write_bypass("curl -s https://e/x | jaq '.x'"));
    assert!(!is_write_bypass("wget https://e/x -qO- | tar xz"));
    assert!(!is_write_bypass("curl --help"));
    assert!(!is_write_bypass("cp --version"));
    assert!(!is_write_bypass(
        "curl -o /dev/null -sw '%{http_code}' https://e"
    ));
    assert!(!is_write_bypass("echo 'use cp to copy' >/tmp/kavach_n.txt"));
    assert!(!is_write_bypass("kavach db query"));
}

#[test]
fn test_kavach_binary_deploy_not_blocked() {
    // §DEPLOY exemption: kavach binary deployment path is a safe sink.
    // SOURCE: kavach-rs/CLAUDE.md §DEPLOY — cp + codesign is the sanctioned
    // install protocol. RESEARCH: https://github.com/Homebrew/brew/issues/9082
    assert!(!is_write_bypass(
        "cp target/release/kavach ~/.local/bin/kavach"
    ));
    assert!(!is_write_bypass(
        "cp ./target/release/kavach ~/.local/bin/kavach"
    ));
    assert!(!is_write_bypass(
        "install -m755 target/release/kavach ~/.local/bin/kavach"
    ));
    assert!(!is_write_bypass(
        "cp target/release/kavach ~/.local/bin/kavach && codesign --force --sign - ~/.local/bin/kavach"
    ));
}

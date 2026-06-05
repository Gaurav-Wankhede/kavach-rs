//! End-to-end regression for the write-bypass DENY. Drives the real
//! `blocklist::check` entry the Bash pre-tool hook calls, proving the exact
//! `python3 - <<HEREDOC` shape that laundered a source edit past the gate is now
//! refused, while a benign Bash write to a generated artifact still passes.

use super::Decision;
use super::check;

/// The exact heredoc that bypassed the pre-write gate this session must DENY.
#[test]
fn python_heredoc_writing_a_rust_source_is_denied() {
    let cmd = "python3 - <<'PY'\nopen('crates/kavach-surreal/src/write.rs','w').write(s)\nPY";
    match check(cmd) {
        Some(Decision::Deny(reason)) => {
            assert!(reason.contains("write-bypass"), "deny reason: {reason}");
        }
        other => panic!(
            "expected Deny for a source heredoc, got {}",
            verdict_name(other.as_ref())
        ),
    }
}

/// A `> file.rs` redirect into the source tree is the same laundering — DENY.
#[test]
fn redirect_into_source_tree_is_denied() {
    assert!(matches!(
        check("echo x > crates/kavach-engine/src/gates/mod.rs"),
        Some(Decision::Deny(_))
    ));
}

/// A Bash write to a generated artifact stays an ADVISORY allow, not a deny —
/// this is the false-positive bound that justifies the P0 promotion.
#[test]
fn bash_write_to_a_generated_artifact_is_allowed_with_advisory() {
    match check("echo data > .config/nextest.toml") {
        Some(Decision::Allow(Some(ctx))) => {
            assert!(ctx.contains("write-bypass"), "advisory ctx: {ctx}");
        }
        // A bare allow (None) is also acceptable — the point is it is NOT a deny.
        Some(Decision::Allow(None)) | None => {}
        Some(Decision::Deny(_)) => panic!("a config write must not be denied"),
        Some(Decision::Ask(_)) => panic!("a config write must not ask"),
    }
}

fn verdict_name(d: Option<&Decision>) -> &'static str {
    match d {
        Some(Decision::Deny(_)) => "Deny",
        Some(Decision::Ask(_)) => "Ask",
        Some(Decision::Allow(_)) => "Allow",
        None => "None",
    }
}

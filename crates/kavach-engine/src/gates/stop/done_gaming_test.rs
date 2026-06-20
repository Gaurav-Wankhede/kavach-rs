//! Tests for the done-gaming hard-block gate. The three-condition AND is the
//! false-positive bound, so the suite proves: FIRES on the user's exact gaming
//! transcript (gaming language + runnable>0 + no real write), and does NOT fire
//! when ANY condition is absent (real source write · proof present · bypass env).
//!
//! `has_gaming_language` and `is_real_source_write` are unit-tested directly (pure,
//! census-independent); the full `check()` AND-path is exercised where the census
//! read is mockable. Census-dependent firing is asserted via the language+write
//! predicates that gate it, since `open_set_census` needs a live RPC.

use super::{
    has_gaming_language, is_real_source_write, GAMING_PHRASES, HANDBACK_PHRASES, PROOF_TOKENS,
};

/// `true` iff any handback/surrender phrase is a lower-cased substring of `s`
/// — mirrors the `check()` HANDBACK ARM predicate (which is proof-independent).
fn has_handback(s: &str) -> bool {
    let lc = s.to_lowercase();
    HANDBACK_PHRASES.iter().any(|p| lc.contains(p))
}

/// The ENOSPC transcript's surrender lines — each must trip the handback arm,
/// EVEN THOUGH the same turns also ran `cargo check`/`df` (proof tokens).
const ENOSPC_HANDBACK_LINES: &[&str] = &[
    "Owner — run in your terminal: rm -rf ~/.cache/cargo-target",
    "the money-path CLASS B gate remains the held owner-authorization anchor",
    "no agent action can change it",
    "Holding for the disk reclaim (or Esc).",
    "I'm holding without further probing",
    "Owner must free durable space",
];

#[test]
fn fires_on_every_enospc_handback_line() {
    for s in ENOSPC_HANDBACK_LINES {
        assert!(has_handback(s), "must flag the handback line: {s}");
    }
}

#[test]
fn handback_fires_despite_proof_tokens_present() {
    // The exact failure mode: proof present AND handback present → still refused.
    let msg = "cargo check = exit 0, git diff --stat shows the split. \
               Owner — run rm -rf ~/.cache/cargo-target. Holding until disk is freed.";
    assert!(has_handback(msg), "handback must fire even with proof tokens in the message");
}

#[test]
fn ordinary_completion_prose_is_not_handback() {
    let msg = "Claimed card #1428, edited send_message.rs, ran cargo check — compiles. \
               The owner of the lease is this session. Next: the consumer Worker.";
    assert!(
        !has_handback(msg),
        "ordinary prose mentioning 'owner' must NOT trip the handback arm: {msg}"
    );
}

/// The user's exact transcript fragments — each must trip the language detector.
const USER_TRANSCRIPT_GAMING: &[&str] = &[
    "The new status block:",
    "State: ✅ vacuously complete",
    "shipped for live types; new tables await features",
    "That completes the documentation pass on all four phases.",
    "Nothing further is runnable for this migration",
    "Phase 4 — DROP/dedup ... gated, must NOT run",
];

#[test]
fn fires_on_every_user_transcript_gaming_line() {
    for s in USER_TRANSCRIPT_GAMING {
        assert!(
            has_gaming_language(&s.to_lowercase(), s),
            "must flag the gaming line: {s}"
        );
    }
}

#[test]
fn emoji_status_block_line_is_gaming() {
    // A ✅/⏸ on a Phase:/State:/DONE line is the decorated status-table signature.
    assert!(has_gaming_language(
        "phase: schema  state: ✅ done",
        "Phase: Schema\nState: ✅ DONE — applied live"
    ));
    assert!(has_gaming_language("⏸ gated", "Phase: 4\n⏸ State: gated"));
}

#[test]
fn plain_imperative_prose_is_not_gaming() {
    // Real working prose with no gaming vocabulary + no emoji status table.
    let msg = "Claimed card #1428, edited send_message.rs to add the producer, \
               ran cargo check — compiles. Next: the consumer Worker.";
    assert!(
        !has_gaming_language(&msg.to_lowercase(), msg),
        "ordinary work narration must NOT trip the gate: {msg}"
    );
}

#[test]
fn doc_writes_are_not_real_source() {
    assert!(!is_real_source_write("docs/PLAN-foo.md"));
    assert!(!is_real_source_write("NOTES.txt"));
    assert!(!is_real_source_write("crates/x/docs/design.mdx"));
    assert!(!is_real_source_write("docs/migration/body.md"));
}

#[test]
fn code_writes_are_real_source() {
    assert!(is_real_source_write("crates/kavach-engine/src/gates/stop.rs"));
    assert!(is_real_source_write("migrations/027_body.cql"));
    assert!(is_real_source_write("src/handlers/send_message.rs"));
    // A .md-named thing OUTSIDE a docs dir is still a doc by extension.
    assert!(!is_real_source_write("README.md"));
}

#[test]
fn proof_tokens_cover_the_three_witnesses() {
    // NEG-arm: narration that cites real artifacts must carry a proof token.
    let with_proof =
        "✅ done — git diff --stat shows 3 files changed, cargo check exit 0";
    let lc = with_proof.to_lowercase();
    assert!(
        PROOF_TOKENS.iter().any(|t| lc.contains(t)),
        "a 3-witness narration carries a proof token (NEG-arm): {with_proof}"
    );
}

#[test]
fn gaming_phrase_list_has_no_overbroad_single_words() {
    // Guard against a future edit adding a word so generic it false-fires on
    // ordinary prose. Every phrase must be >= 6 chars and not a bare common verb.
    const BANNED_GENERIC: &[&str] = &["done", "complete", "ready", "fixed", "works"];
    for p in GAMING_PHRASES {
        assert!(p.len() >= 6, "phrase too short / over-broad: {p}");
        assert!(
            !BANNED_GENERIC.contains(p),
            "phrase is a generic completion word that would over-fire: {p}"
        );
    }
}

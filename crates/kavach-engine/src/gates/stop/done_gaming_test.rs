//! Tests for the done-gaming hard-block gate. The three-condition AND is the
//! false-positive bound, so the suite proves: FIRES on the user's exact gaming
//! transcript (gaming language + runnable>0 + no real write), and does NOT fire
//! when ANY condition is absent (real source write · proof present · bypass env).
//!
//! The vocabulary is now DB-sourced ([`DoneGamingVocab`], `gate.done_gaming_vocab`),
//! so the suite also proves the DYNAMIC contract: the compiled `Default` still
//! matches the user transcript (fail-open floor), AND a DB-shaped override changes
//! the verdict (the markers are DATA, not literals). `has_gaming_language` and
//! `is_real_source_write` are unit-tested directly; census-dependent firing is
//! asserted via the predicates that gate it (`open_set_census` needs a live RPC).

use kavach_patterns::stop_vocab::DoneGamingVocab;

use super::{PROOF_TOKENS, has_gaming_language, is_real_source_write};

/// `true` iff any handback phrase in the DEFAULT vocab is a lower-cased substring
/// of `s` — mirrors the `check()` HANDBACK ARM predicate (proof-independent).
fn has_handback(s: &str) -> bool {
    DoneGamingVocab::default().has_handback_phrase(&s.to_lowercase())
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
    assert!(
        has_handback(msg),
        "handback must fire even with proof tokens in the message"
    );
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

/// The user's exact transcript fragments — each must trip the language detector
/// under the compiled DEFAULT vocab (the fail-open floor when the DB is down).
const USER_TRANSCRIPT_GAMING: &[&str] = &[
    "The new status block:",
    "State: ✅ vacuously complete",
    "shipped for live types; new tables await features",
    "That completes the documentation pass on all four phases.",
    "Nothing further is runnable for this migration",
    "Phase 4 — DROP/dedup ... gated, must NOT run",
];

#[test]
fn default_vocab_fires_on_every_user_transcript_gaming_line() {
    let vocab = DoneGamingVocab::default();
    for s in USER_TRANSCRIPT_GAMING {
        assert!(
            has_gaming_language(&vocab, &s.to_lowercase(), s),
            "DEFAULT (floor) vocab must flag the gaming line: {s}"
        );
    }
}

#[test]
fn emoji_status_block_line_is_gaming() {
    // A ✅/⏸ on a Phase:/State:/DONE line is the decorated status-table signature —
    // matched structurally, independent of the (DB-sourced) phrase list.
    let vocab = DoneGamingVocab::default();
    assert!(has_gaming_language(
        &vocab,
        "phase: schema  state: ✅ done",
        "Phase: Schema\nState: ✅ DONE — applied live"
    ));
    assert!(has_gaming_language(
        &vocab,
        "⏸ gated",
        "Phase: 4\n⏸ State: gated"
    ));
}

#[test]
fn plain_imperative_prose_is_not_gaming() {
    let vocab = DoneGamingVocab::default();
    let msg = "Claimed card #1428, edited send_message.rs to add the producer, \
               ran cargo check — compiles. Next: the consumer Worker.";
    assert!(
        !has_gaming_language(&vocab, &msg.to_lowercase(), msg),
        "ordinary work narration must NOT trip the gate: {msg}"
    );
}

// --- DYNAMIC contract: the vocab is DATA, sourced from the DB, not literals ------

#[test]
fn config_is_data_db_override_changes_the_gaming_verdict() {
    // A DB row shaping a NEW phrase (deserialized exactly as `done_gaming_vocab_for`
    // does) must make the gate fire on text the compiled floor would pass.
    let row = r#"{"gaming_phrases":["mission accomplished"]}"#;
    let vocab: DoneGamingVocab = serde_json::from_str(row).expect("valid override");
    let msg = "mission accomplished — closing out.";
    assert!(
        vocab.has_gaming_phrase(&msg.to_lowercase()),
        "DB-overridden phrase must drive the verdict (markers are data)"
    );
    // And `#[serde(default)]` keeps the handback floor the row omitted.
    assert!(
        vocab.has_handback_phrase("i am holding for the disk reclaim"),
        "omitted handback list must fall back to the compiled floor"
    );
}

#[test]
fn malformed_db_row_degrades_to_the_default_floor() {
    // Mirrors `done_gaming_vocab_for`'s `unwrap_or_default()`: a malformed blob is
    // never a panic and never an empty vocab — it is the full compiled floor.
    let vocab: DoneGamingVocab = serde_json::from_str("{ not json").unwrap_or_default();
    assert!(vocab.has_gaming_phrase("vacuously complete"));
    assert!(vocab.has_handback_phrase("i am holding"));
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
    assert!(is_real_source_write(
        "crates/kavach-engine/src/gates/stop.rs"
    ));
    assert!(is_real_source_write("migrations/027_body.cql"));
    assert!(is_real_source_write("src/handlers/send_message.rs"));
    // A .md-named thing OUTSIDE a docs dir is still a doc by extension.
    assert!(!is_real_source_write("README.md"));
}

#[test]
fn proof_tokens_cover_the_three_witnesses() {
    let with_proof = "✅ done — git diff --stat shows 3 files changed, cargo check exit 0";
    let lc = with_proof.to_lowercase();
    assert!(
        PROOF_TOKENS.iter().any(|t| lc.contains(t)),
        "a 3-witness narration carries a proof token (NEG-arm): {with_proof}"
    );
}

#[test]
fn default_gaming_phrases_have_no_overbroad_single_words() {
    // Guard against a future floor edit adding a word so generic it false-fires on
    // ordinary prose. Every default phrase must be >= 6 chars and not a bare verb.
    const BANNED_GENERIC: &[&str] = &["done", "complete", "ready", "fixed", "works"];
    for p in &DoneGamingVocab::default().gaming_phrases {
        assert!(p.len() >= 6, "phrase too short / over-broad: {p}");
        assert!(
            !BANNED_GENERIC.contains(&p.as_str()),
            "phrase is a generic completion word that would over-fire: {p}"
        );
    }
}

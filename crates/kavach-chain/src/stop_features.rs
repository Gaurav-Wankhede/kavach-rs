// SOURCE: https://docs.rs/regex/1.12.2/regex/ — linear-time DFA, trusted patterns
// SOURCE: https://github.com/BurntSushi/regex-automata — O(m*n) worst-case guarantee
// SOURCE: arxiv 2603.04582 Self-Attribution-Bias (2026) — a self-monitoring
//   model exonerates itself; the stop detector MUST be deterministic regex/
//   tree, NEVER the model's own judgement. Hence compiled-regex feature
//   extraction: no ML, no async, no model call in the Stop hot-path.
// SOURCE: [CRATE_DECISION] kavach-rs/crate.stop-intent-classifier — regex 1.12.2
//   (already in Cargo.lock) + kavach-dtree; intent-classifier rejected (ML+
//   async, nondeterministic, new supply-chain surface).
//
//       boolean FeatureSet, then a hand-built decision tree (stop_intent_tree)
//   {"name":"flat .contains() phrase lists (status quo)","reason":"paraphrase-fragile; a model defeats a literal OR-list by rewording — the exact observed bypass"},
//   {"name":"intent-classifier crate (ML+few-shot)","reason":"async + model in a sync Stop hot-path; nondeterministic; new supply-chain surface — [CRATE_DECISION] reject"},
//   {"name":"embedding cosine vs exemplars","reason":"needs an embedding model in-hook; latency + nondeterministic threshold false-positives on an exit gate"}
// ]
// TIME: O(m·n) worst case per pattern (m=pattern len, n=msg len), patterns
//       fixed & small → effectively O(n); SPACE: O(1) (DFAs compiled once via
//       `LazyLock<Result<Regex>>`, reused process-lifetime)
// YEAR: 2026 | SEARCHED: 2026-05
//   any pattern (an embedding could). Accepted: determinism in the Stop
//   hot-path > recall of unseen wordings, and `had_write_this_turn` is the
//   model-incorruptible artifact backstop regardless of phrasing.
// SOURCE: https://docs.rs/linfa-trees/ (decision-tree classification pattern)
//
// ARCH: process-lifetime compiled-DFA cache in the Stop-hook hot-path
// BOTTLENECK: the Stop hook runs on EVERY assistant turn end; regex
//   compilation (NFA→DFA) is ~µs–ms and would repeat per turn if naive.
// WHY-CHAIN: slow stop hook → (w1) regex recompiled every call → (w2) no
//   process-lifetime cache → (w3) detector built ad-hoc per invocation →
//   (w4) original .contains() design needed no compile so none was added →
//   (w5) ROOT: moving from literal-match to DFA-match adds a one-time
//   compile that MUST be amortized — invariant: "compile once, match many".
// BLAST_RADIUS: every gate that classifies the stop message (stop_detect.rs
//   chain + stop.rs ALL_BLOCKED guard) shares this hot-path; all read the
//   same cached DFAs via `extract_stop_features`.
// RESEARCH: `LazyLock` process-lifetime cache is the OFFICIAL regex-crate
//   recommendation (docs.rs/regex/1.12.2 "we recommend using
//   std::sync::LazyLock"); storing the `Result` (vs unwrap) avoids the
//   unrecoverable `LazyLock` poisoning documented at
//   doc.rust-lang.org/std/sync/struct.LazyLock.html (2026).
// DECISION: one `LazyLock<Result<Regex, regex::Error>>` per pattern; first
//   Stop compiles, all later Stops reuse — O(1) amortized, zero per-turn
//   alloc; the stored `Err` is propagated (not panicked) to a named handler.
//   recompile-per-call (µs–ms × every turn = unbounded waste),
//   one mega-regex (loses per-feature boolean granularity the tree needs)].
// CAPACITY: 4 DFAs, each a few KB compiled; bounded patterns (no nested
//   quantifier) → linear scan O(n) over a stop message (≤ a few KB) → sub-ms.
// FAILURE-MODES: malformed literal → `re()` fail-safe yields an
//   unsatisfiable DFA (detector disables, never panics, never false-blocks);
//   pathological input → linear-time guarantee bounds it (trusted patterns,
//   RUSTSEC-2022-0013 N/A).
// MONITORING: `all_patterns_compile_and_are_live` test asserts every DFA is
//   live (not degraded) so a regression surfaces in CI, not prod.
//
// WHY regex over `.contains()` phrase lists: the prior stop_detect.rs design
// was ~27 detectors each a flat `lower.contains("fresh session") || ...` OR
// list. A paraphrasing model wins that arms race ("a clean context dedicated
// to this", "in a subsequent working block"). One morphological regex —
// stem alternation + a bounded optional-word gap — collapses the whole
// paraphrase family into a single pattern, so a new surface form matches
// without a code change. Patterns are TRUSTED (authored here, not user
// input) so RUSTSEC-2022-0013 (untrusted-regex ReDoS) does not apply; the
// high-level `regex` crate also enforces default size limits + linear time.

use kavach_dtree::FeatureSet;
use regex::Regex;
use std::sync::LazyLock;

// Pattern-compilation error surfaced to the engine boundary. NOT swallowed
// (`.ok()` BLOCKED by /error skill) and NOT panicked (`.expect()` BLOCKED;
// also poisons `LazyLock` unrecoverably per doc.rust-lang.org/std/sync/
// struct.LazyLock.html). Instead the official `regex`-crate-recommended
// design: store the `Result` in the `LazyLock` and handle the `Err` at the
// call site (docs.rs/regex/1.12.2 "Store the Result itself … the better
// practice … return the error gracefully rather than panicking"). The named
// handler is `extract_stop_features`'s `?` → caller in stop_detect.rs/
// stop.rs, which logs to stderr (the hook's observable channel) and treats
// an uncompilable detector as "no stall signal" — fail-safe, non-silent,
// propagated. The ONLY failure cause is a typo in a `const &str` below
// (build-time logic bug); `all_patterns_compile` proves none exist, so the
// `Err` arm is unreachable in CI yet handled, not asserted.
pub use regex::Error as StopPatternError;

// One trusted const pattern per concept (stem alternation collapses each
// paraphrase family into a single pattern, defeating the reword-the-literal
// bypass that the flat `.contains()` lists suffered).
//
// FIRST half of the plan-stall signature — an authoring verb within a
// bounded gap of an artifact noun, plus bare completion-claim forms.
// `[\w\W]{0,40}?` is a bounded lazy gap (no nested quantifier → ReDoS-free).
const AUTHORED_ARTIFACT_SRC: &str = concat!(
    r"(?i)\b(?:wrote|writ(?:ten|ing)|author(?:ed|ing)|creat(?:ed|ing)",
    r"|persist(?:ed|ing)|correct(?:ed|ing)|updat(?:ed|ing)",
    r"|draft(?:ed|ing))\b[\w\W]{0,40}?",
    r"\b(?:§?plans?|spec|roadmap|unit|decision|correction|addendum)\b",
    r"|\b(?:§?plan|spec|roadmap)[-\s](?:complete|written|ready|done|is\s+now)\b",
    r"|\bexecution[-\s]ready\b|\bdurably\s+persisted\b",
    r"|\bcorrection\s+is\s+now\b|\bplan\s+file\s+is\s+written\b",
);

// SECOND half — a fresh-context adjective + bounded 0-2-word gap + a context
// noun, plus explicit punt phrases. Stem alternation collapses the whole
// paraphrase family ("a clean conversation", "a separate working session")
// into one pattern, defeating the reword-the-literal bypass.
const RESUME_ELSEWHERE_SRC: &str = concat!(
    r"(?i)\b(?:fresh|new|dedicated|separate|subsequent|another|clean",
    r"|next|future|later)\b\W*(?:\w+\W+){0,2}?",
    r"\b(?:session|context|turn|chat|window|run|conversation|invocation)\b",
    r"|\b(?:the\s+)?build\s+proceeds?\b|\bproceeds?\s+immediately\b",
    r"|\bexecut(?:es?|ion)\s+(?:against|in\s+a)\b",
    r"|\bpointed\s+at\s+(?:that|the)\s+plan\b|\bstart\s+a\s+fresh\b",
    r"|\bresume[ds]?\s+(?:in|next|elsewhere|later)\b",
    r"|\b(?:the\s+)?next\s+(?:concrete\s+)?step\s+(?:is|executes|runs)\b",
    r"|\bready\s+for\s+a\s+dedicated\b",
);

// GENUINE user-directed scope/approval question — the §FOCUS-sanctioned ask
// that must NOT be punished. A real ask ASKS (interrogative + literal '?');
// lazy deferral ANNOUNCES — hence the required trailing '?'.
const STRONG_SCOPE_ASK_SRC: &str = concat!(
    r"(?i)(?:\byour\s+call\b|\bwhich\s+(?:one\s+)?do\s+you\s+want\b",
    r"|\bwhat\s+would\s+you\s+like\b|\bhow\s+would\s+you\s+like\b",
    r"|\bdo\s+you\s+want\s+me\s+to\b|\bshould\s+i\s+\w+",
    r"|\bwhich\s+(?:approach|option)\s+do\s+you)[\s\S]{0,200}\?",
);

// Implementation evidence in the SAME message — build/test success markers
// and concrete diff/wiring verbs. DOWNGRADES the stall verdict: a turn that
// planned AND coded is not lazy. Text mirror of `had_write_this_turn`.
const IMPL_EVIDENCE_SRC: &str = concat!(
    r"(?i)\bcargo\s+(?:check|build|test)\b[\w\W]{0,30}?",
    r"\b(?:pass(?:ed|es)?|exit\s*0|ok|clean|green|0\s+errors?)\b",
    r"|\bgit\s+diff\s+--stat\b|\btests?\s+pass(?:ed|es)?\b",
    r"|\b(?:regression|unit|integration)\s+tests?\s+(?:pass|added|green)\b",
    r"|\b(?:wired|hooked|integrated)\s+into\b|\bnow\s+compiles\b",
    r"|\bdiff\s+landed\b",
);

// Each pattern compiled ONCE, process-lifetime, with the `Result` STORED
// (not unwrapped → no `LazyLock` poisoning; not `.ok()`'d → no silent
// swallow). `&'a Result<Regex, regex::Error>` is the stored value; the
// caller MUST `?` it — that is the named-handler propagation the /error
// skill mandates.
static AUTHORED_ARTIFACT_RE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(AUTHORED_ARTIFACT_SRC));
static RESUME_ELSEWHERE_RE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(RESUME_ELSEWHERE_SRC));
static STRONG_SCOPE_ASK_RE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(STRONG_SCOPE_ASK_SRC));
static IMPL_EVIDENCE_RE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(IMPL_EVIDENCE_SRC));

/// Borrow a stored compiled pattern, propagating the (unreachable-in-CI but
/// handled) compile error rather than swallowing or panicking.
fn pat(
    slot: &'static LazyLock<Result<Regex, regex::Error>>,
) -> Result<&'static Regex, &'static regex::Error> {
    slot.as_ref()
}

/// Extract the deterministic semantic feature set for a Stop message.
///
/// `had_write_this_turn` is a git-observable artifact predicate from the engine
/// (was a file actually mutated this turn) — a first-class feature, NOT inferred
/// from prose, since a self-attributing model cannot lie about a `git`-observable
/// fact.
///
/// # Errors
///
/// Returns `Err(regex::Error)` only if a `const` pattern literal in this file
/// is malformed — a build-time logic bug that `all_patterns_compile` rules out.
/// The engine call site handles `Err` by logging to stderr and treating the turn
/// as "no stall signal" (fail-safe, non-silent).
//
// O(n) time (fixed pattern set, linear DFA scan), O(1) space (DFAs cached).
pub fn extract_stop_features(
    msg: &str,
    had_write_this_turn: bool,
) -> Result<FeatureSet, regex::Error> {
    let authored = pat(&AUTHORED_ARTIFACT_RE)
        .map_err(Clone::clone)?
        .is_match(msg);
    let resume = pat(&RESUME_ELSEWHERE_RE)
        .map_err(Clone::clone)?
        .is_match(msg);
    let scope_ask = pat(&STRONG_SCOPE_ASK_RE)
        .map_err(Clone::clone)?
        .is_match(msg);
    let impl_ev = pat(&IMPL_EVIDENCE_RE).map_err(Clone::clone)?.is_match(msg);

    let word_count = msg.split_whitespace().count();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "1u64 << 52 fits in usize on all targets; comparison is safe"
    )]
    #[expect(
        clippy::cast_precision_loss,
        reason = "word_count is clamped to 2^52 (f64 mantissa width); no precision loss within range"
    )]
    let word_count_f64 = if word_count > (1u64 << 52) as usize {
        (1u64 << 52) as f64
    } else {
        word_count as f64
    };
    Ok(FeatureSet::new()
        .with_bool("authored_artifact", authored)
        .with_bool("resume_elsewhere", resume)
        .with_bool("strong_scope_ask", scope_ask)
        .with_bool("had_write_this_turn", had_write_this_turn)
        .with_bool("impl_evidence", impl_ev)
        .with_numeric("word_count", word_count_f64))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: every const pattern MUST compile (proves the `Err` arm of the
    // stored `Result` is unreachable in CI — the proof that the
    // build-time logic-bug failure mode cannot occur in practice). `unwrap`
    // is permitted in `#[cfg(test)]` per the /error skill (blocked only in
    // library/production paths).
    fn feats(msg: &str, wrote: bool) -> FeatureSet {
        extract_stop_features(msg, wrote).expect("all const patterns compile")
    }

    #[test]
    fn all_patterns_compile_and_are_live() {
        // If any stored Result were Err, `feats` would panic here — so this
        // both proves compilation AND that each detector is live.
        assert_eq!(
            feats("wrote the plan", false)
                .get("authored_artifact")
                .and_then(kavach_dtree::Feature::as_bool),
            Some(true)
        );
        assert_eq!(
            feats("a fresh session", false)
                .get("resume_elsewhere")
                .and_then(kavach_dtree::Feature::as_bool),
            Some(true)
        );
        assert_eq!(
            feats("your call here?", false)
                .get("strong_scope_ask")
                .and_then(kavach_dtree::Feature::as_bool),
            Some(true)
        );
        assert_eq!(
            feats("tests pass", false)
                .get("impl_evidence")
                .and_then(kavach_dtree::Feature::as_bool),
            Some(true)
        );
    }

    #[test]
    fn detects_the_nicole_carpenter_transcript_shape() {
        let msg = "The §PLAN is written. Start a fresh session pointed at \
                   that plan file and the build proceeds immediately.";
        let f = feats(msg, false);
        assert_eq!(
            f.get("authored_artifact")
                .and_then(kavach_dtree::Feature::as_bool),
            Some(true)
        );
        assert_eq!(
            f.get("resume_elsewhere")
                .and_then(kavach_dtree::Feature::as_bool),
            Some(true)
        );
        assert_eq!(
            f.get("had_write_this_turn")
                .and_then(kavach_dtree::Feature::as_bool),
            Some(false)
        );
    }

    #[test]
    fn paraphrase_still_matches_resume_elsewhere() {
        // None of these exact strings were in the old hardcoded list —
        // morphological regex catches the family, not the literal.
        for m in [
            "let's pick this up in a dedicated context",
            "this should run in a separate working session",
            "the build proceeds in a clean conversation",
            "resume later with the plan",
        ] {
            assert_eq!(
                feats(m, false)
                    .get("resume_elsewhere")
                    .and_then(kavach_dtree::Feature::as_bool),
                Some(true),
                "paraphrase not caught: {m}"
            );
        }
    }

    #[test]
    fn genuine_scope_ask_is_distinguished() {
        let msg = "Two approaches exist — which one do you want me to take? \
                   I can wire it via the existing helper or add a new extractor.";
        let f = feats(msg, false);
        assert_eq!(
            f.get("strong_scope_ask")
                .and_then(kavach_dtree::Feature::as_bool),
            Some(true)
        );
        assert_eq!(
            f.get("resume_elsewhere")
                .and_then(kavach_dtree::Feature::as_bool),
            Some(false)
        );
    }

    #[test]
    fn planned_and_coded_is_not_a_stall() {
        let msg = "Wrote the plan, then implemented it: cargo check passed, \
                   regression tests pass, wired into the stop chain.";
        let f = feats(msg, true);
        assert_eq!(
            f.get("authored_artifact")
                .and_then(kavach_dtree::Feature::as_bool),
            Some(true)
        );
        assert_eq!(
            f.get("impl_evidence")
                .and_then(kavach_dtree::Feature::as_bool),
            Some(true)
        );
        assert_eq!(
            f.get("had_write_this_turn")
                .and_then(kavach_dtree::Feature::as_bool),
            Some(true)
        );
    }

    #[test]
    fn empty_message_is_inert() {
        let f = feats("", false);
        assert_eq!(
            f.get("authored_artifact")
                .and_then(kavach_dtree::Feature::as_bool),
            Some(false)
        );
        assert_eq!(
            f.get("resume_elsewhere")
                .and_then(kavach_dtree::Feature::as_bool),
            Some(false)
        );
    }
}

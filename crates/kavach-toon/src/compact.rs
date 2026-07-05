// SOURCE: github.com/JuliusBrussee/compact README (fetched 2026-07-06)
mod preserve;
mod rules;
mod verify;

use thiserror::Error;

/// Compression aggressiveness for grammar-dropping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(clippy::exhaustive_enums, reason = "public compression level DTO")]
pub enum Level {
    Lite,
    Full,
    Ultra,
}

/// Error returned when a preserved span is lost during compression.
#[derive(Debug, Error)]
#[expect(clippy::exhaustive_enums, reason = "cross-crate error")]
pub enum CompactError {
    #[error("preserved span dropped: {0}")]
    PreservedTokenDropped(String),
}

/// Drop conversational grammar, preserving code/paths/URLs/signal-tokens byte-for-byte.
#[must_use]
pub fn compress(text: &str, level: Level) -> String {
    let spans = preserve::preserved_spans(text);
    let (masked, originals) = preserve::mask(text, &spans);
    let dropped = rules::drop_grammar(&masked, level);
    preserve::unmask(&dropped, &originals)
}

#[expect(clippy::missing_errors_doc, reason = "single-sentence summary above covers it")]
pub fn assert_lossless(original: &str, compressed: &str) -> Result<(), CompactError> {
    verify::check_lossless(original, compressed)
}

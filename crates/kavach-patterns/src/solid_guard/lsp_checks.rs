//! Liskov Substitution Principle (LSP) violation detection.

use crate::solid_guard::{SolidLetter, SolidSeverity, SolidViolation};

fn push(
    v: &mut Vec<SolidViolation>,
    severity: SolidSeverity,
    pattern: &'static str,
    fix: &'static str,
) {
    v.push(SolidViolation {
        severity,
        letter: SolidLetter::L,
        pattern,
        fix,
        line: 0,
    });
}

pub(super) fn check_panic_in_trait_impl(
    p: &[regex::Regex],
    content: &str,
    v: &mut Vec<SolidViolation>,
) {
    if p.get(4).is_some_and(|re| re.is_match(content)) {
        push(
            v,
            SolidSeverity::P1Advisory,
            "lsp-panic-in-trait-impl",
            "panic!/unimplemented!/todo! inside trait impl breaks substitutability. Either return Err, or split capability into a separate trait.",
        );
    }
}

pub(super) fn check_result_then_unwrap(
    p: &[regex::Regex],
    content: &str,
    v: &mut Vec<SolidViolation>,
) {
    if p.get(5).is_some_and(|re| re.is_match(content)) {
        push(
            v,
            SolidSeverity::P2Warning,
            "lsp-result-then-unwrap",
            "fn signature promises Result but body unwraps internally. Caller cannot recover; propagate error with ?.",
        );
    }
}

pub(super) fn check_block_on_in_trait_impl(
    p: &[regex::Regex],
    content: &str,
    v: &mut Vec<SolidViolation>,
) {
    if p.get(19).is_some_and(|re| re.is_match(content)) {
        push(
            v,
            SolidSeverity::P1Advisory,
            "lsp-block-on-in-trait-impl",
            "block_on inside trait impl sync-blocks the executor — async substitutes deadlock the runtime. Make the trait method async (#[async_trait]).",
        );
    }
}

pub(super) fn check_axum_lsp_service_panic(
    p: &[regex::Regex],
    content: &str,
    v: &mut Vec<SolidViolation>,
) {
    if p.get(14).is_some_and(|re| re.is_match(content)) {
        push(
            v,
            SolidSeverity::P1Advisory,
            "axum-lsp-service-panic",
            "tower::Service impl with panic!/unimplemented!/todo! crashes the worker and breaks substitutability. Return Err via the associated Error type.",
        );
    }
}

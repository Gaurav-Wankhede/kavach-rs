//! Open/Closed Principle (OCP) violation detection.

use crate::solid_guard::{SolidLetter, SolidSeverity, SolidViolation};

fn push(
    v: &mut Vec<SolidViolation>,
    severity: SolidSeverity,
    pattern: &'static str,
    fix: &'static str,
) {
    v.push(SolidViolation {
        severity,
        letter: SolidLetter::O,
        pattern,
        fix,
        line: 0,
    });
}

pub(super) fn check_provider_match(p: &[regex::Regex], content: &str, v: &mut Vec<SolidViolation>) {
    if p.get(2).is_some_and(|re| re.is_match(content)) {
        push(
            v,
            SolidSeverity::P1Advisory,
            "ocp-provider-match",
            "match arms on concrete providers violate OCP. Define a trait + dyn dispatch; add new provider via new impl, not new arm.",
        );
    }
}

pub(super) fn check_string_dispatch(
    p: &[regex::Regex],
    content: &str,
    v: &mut Vec<SolidViolation>,
) {
    if p.get(3).is_some_and(|re| re.is_match(content)) {
        push(
            v,
            SolidSeverity::P1Advisory,
            "ocp-string-dispatch",
            "String equality dispatch is closed-for-extension. Replace with trait object lookup keyed by provider id.",
        );
    }
}

pub(super) fn check_policy_with_vendor_switch(
    p: &[regex::Regex],
    content: &str,
    v: &mut Vec<SolidViolation>,
) {
    if p.get(11).is_some_and(|re| re.is_match(content)) {
        push(
            v,
            SolidSeverity::P1Advisory,
            "ocp-policy-with-vendor-switch",
            "Business policy fn switches on vendor enum. Inject Box<dyn Trait> instead; OCP says: extend, don't modify.",
        );
    }
}

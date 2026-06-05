//! Interface Segregation Principle (ISP) violation detection.

use super::helpers;
use crate::solid_guard::{SolidLetter, SolidSeverity, SolidViolation};

fn push(
    v: &mut Vec<SolidViolation>,
    severity: SolidSeverity,
    pattern: &'static str,
    fix: &'static str,
) {
    v.push(SolidViolation {
        severity,
        letter: SolidLetter::I,
        pattern,
        fix,
        line: 0,
    });
}

pub(super) fn check_fat_trait(p: &[regex::Regex], content: &str, v: &mut Vec<SolidViolation>) {
    if let Some(re) = p.get(6) {
        for cap in re.captures_iter(content) {
            if cap
                .get(1)
                .is_some_and(|b| helpers::count_trait_methods(b.as_str()) > 7)
            {
                push(
                    v,
                    SolidSeverity::P1Advisory,
                    "isp-fat-trait",
                    "trait has >7 methods. Split into role traits (Reader/Writer/Deleter); compose with `T: Reader + Writer`.",
                );
                break;
            }
        }
    }
}

pub(super) fn check_storage_god_trait(
    p: &[regex::Regex],
    content: &str,
    v: &mut Vec<SolidViolation>,
) {
    if p.get(7).is_some_and(|re| re.is_match(content)) {
        push(
            v,
            SolidSeverity::P1Advisory,
            "isp-storage-god-trait",
            "Single trait combines get+put+delete. Split into Reader/Writer/Deleter; read-only consumers shouldn't depend on write API.",
        );
    }
}

pub(super) fn check_catchall_method(
    p: &[regex::Regex],
    content: &str,
    v: &mut Vec<SolidViolation>,
) {
    if p.get(12).is_some_and(|re| re.is_match(content)) {
        push(
            v,
            SolidSeverity::P2Warning,
            "isp-catchall-method",
            "fn name (do_everything/handle_all/process_all) signals missing decomposition. Name a single capability per method.",
        );
    }
}

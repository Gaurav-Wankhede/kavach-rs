//! Single Responsibility Principle (SRP) violation detection.

use super::helpers;
use super::pattern_strs;
use crate::solid_guard::{SolidLetter, SolidSeverity, SolidViolation};

fn push(
    v: &mut Vec<SolidViolation>,
    severity: SolidSeverity,
    pattern: &'static str,
    fix: &'static str,
) {
    v.push(SolidViolation {
        severity,
        letter: SolidLetter::S,
        pattern,
        fix,
        line: 0,
    });
}

pub(super) fn check_god_struct(p: &[regex::Regex], content: &str, v: &mut Vec<SolidViolation>) {
    if let Some(re) = p.get(1) {
        for cap in re.captures_iter(content) {
            if cap
                .get(1)
                .is_some_and(|b| helpers::count_struct_fields(b.as_str()) > 8)
            {
                push(
                    v,
                    SolidSeverity::P1Advisory,
                    "srp-god-struct",
                    "Struct has >8 fields. Split into focused sub-structs (DDD value objects). One reason to change per type.",
                );
                break;
            }
        }
    }
}

pub(super) fn check_long_async_fn(p: &[regex::Regex], content: &str, v: &mut Vec<SolidViolation>) {
    if let Some(re) = p.get(10) {
        for m in re.find_iter(content) {
            if helpers::count_lines_in_async_fn(content, m.end()) > 80 {
                push(
                    v,
                    SolidSeverity::P1Advisory,
                    "srp-long-async-fn",
                    "async fn body >80 lines. Extract use-case orchestration into application service; keep handler thin.",
                );
                break;
            }
        }
    }
}

pub(super) fn check_conflated_derives(
    p: &[regex::Regex],
    content: &str,
    v: &mut Vec<SolidViolation>,
) {
    if let Some(re) = p.get(17) {
        for cap in re.captures_iter(content) {
            if cap.get(1).is_some_and(|b| {
                helpers::count_conflated_derives(b.as_str(), pattern_strs::CONFLATED_DERIVES) >= 3
            }) {
                push(
                    v,
                    SolidSeverity::P1Advisory,
                    "srp-conflated-derives",
                    "Struct conflates 3+ of {FromRow, Serialize, Deserialize, ToSchema, sqlx::Type}. Split into Row → Entity → Dto → ApiSchema.",
                );
                break;
            }
        }
    }
}

pub(super) fn check_handler_builds_router(
    p: &[regex::Regex],
    content: &str,
    v: &mut Vec<SolidViolation>,
) {
    if p.get(15).is_some_and(|re| re.is_match(content)) {
        push(
            v,
            SolidSeverity::P1Advisory,
            "axum-srp-handler-builds-router",
            "Handler body contains Router::new() — handler is also doing wiring. Move route construction to a dedicated app::router() fn.",
        );
    }
}

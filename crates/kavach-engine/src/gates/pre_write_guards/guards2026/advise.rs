//! Advisory-only 2026 detectors — never block. Grouped to match the original
//! interleaved dispatch order around the severity-routed guards.
use super::super::result::Acc;
use crate::gates::pre_write_context::WriteContext;

/// SOLID + DSA + system-design advisories (run first).
pub(super) fn solid_dsa_design(ctx: &WriteContext<'_>, acc: &mut Acc) {
    let p1 = &mut acc.p1_advisories;
    for v in kavach_patterns::solid_guard::detect(ctx.file_path, ctx.content) {
        p1.push(format!(
            "[SOLID_{:?}_P1] {}: {}",
            v.letter, v.pattern, v.fix
        ));
    }
    for v in kavach_patterns::dsa_guard::detect(ctx.file_path, ctx.content) {
        p1.push(format!("[DSA_{:?}_P1] {}: {}", v.class, v.pattern, v.fix));
    }
    for v in kavach_patterns::system_design_guard::detect(ctx.file_path, ctx.content) {
        p1.push(format!("[SYSDESIGN_P1] {}: {}", v.pattern, v.fix));
    }
}

/// Atomic-UI advisories (run before `rust_196`).
pub(super) fn atomic_ui(ctx: &WriteContext<'_>, acc: &mut Acc) {
    for v in kavach_patterns::atomic_ui_guard::detect(ctx.file_path, ctx.content) {
        acc.p1_advisories
            .push(format!("[ATOMIC_UI_P1] {}: {}", v.pattern, v.fix));
    }
}

/// Dioxus + Axum advisories (run between `rust_196` and `async_sync`).
pub(super) fn dioxus_axum(ctx: &WriteContext<'_>, acc: &mut Acc) {
    let p1 = &mut acc.p1_advisories;
    for v in kavach_patterns::dioxus_guard::detect(ctx.file_path, ctx.content) {
        p1.push(format!("[DIOXUS_P1] {}: {}", v.pattern, v.fix));
    }
    for v in kavach_patterns::axum_guard::detect(ctx.file_path, ctx.content) {
        p1.push(format!("[AXUM_P1] {}: {}", v.pattern, v.fix));
    }
}

/// API-management + design-patterns advisories (run between `async_sync` and `db_ops`).
pub(super) fn api_mgmt_design(ctx: &WriteContext<'_>, acc: &mut Acc) {
    let p1 = &mut acc.p1_advisories;
    for v in kavach_patterns::design_patterns_guard::detect(ctx.file_path, ctx.content) {
        p1.push(format!("[DESIGN_P1] {}: {}", v.pattern, v.fix));
    }
}

/// Observability + finops advisories (run between `db_ops` and pii).
pub(super) fn observability_finops(ctx: &WriteContext<'_>, acc: &mut Acc) {
    let p1 = &mut acc.p1_advisories;
    for v in kavach_patterns::observability_guard::detect(ctx.file_path, ctx.content) {
        p1.push(format!("[OBS_P1] {}: {}", v.pattern, v.fix));
    }
    for v in kavach_patterns::finops_guard::detect(ctx.file_path, ctx.content) {
        p1.push(format!("[FINOPS_P1] {}: {}", v.pattern, v.fix));
    }
}

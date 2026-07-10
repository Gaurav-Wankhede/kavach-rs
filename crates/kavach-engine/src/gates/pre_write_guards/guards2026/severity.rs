//! Severity-routed 2026 detectors: `P0Block` returns a block reason; P1/P2 push
//! advisories. Each fn preserves the original per-guard dispatch semantics.
use super::super::result::Acc;
use crate::gates::pre_write_context::WriteContext;

/// `rust_196` — `P0Block` hard-blocks; P1/P2 advise.
pub(super) fn rust_196(ctx: &WriteContext<'_>, acc: &mut Acc) -> Option<String> {
    use kavach_patterns::rust_196_guard::Rust196Severity::{P0Block, P1Advisory, P2Warning};
    for v in kavach_patterns::rust_196_guard::detect(ctx.file_path, ctx.content) {
        match v.severity {
            P0Block => return Some(format!("[RUST196_P0/{}] {}", v.pattern, v.fix)),
            P1Advisory => acc
                .p1_advisories
                .push(format!("[RUST196_P1] {}: {}", v.pattern, v.fix)),
            P2Warning => acc
                .p1_advisories
                .push(format!("[RUST196_P2] {}: {}", v.pattern, v.fix)),
        }
    }
    None
}

/// async/sync — `P0Block` on `std::sync::Mutex` / `std::thread::sleep` in async.
pub(super) fn async_sync(ctx: &WriteContext<'_>, acc: &mut Acc) -> Option<String> {
    use kavach_patterns::async_sync_guard::AsyncSeverity::{P0Block, P1Advisory, P2Warning};
    for v in &kavach_patterns::async_sync_guard::detect(ctx.file_path, ctx.content) {
        match v.severity {
            P0Block => return Some(format!("[ASYNC_P0/{}] {}", v.pattern, v.fix)),
            P1Advisory => acc
                .p1_advisories
                .push(format!("[ASYNC_P1] {}: {}", v.pattern, v.fix)),
            P2Warning => acc
                .p1_advisories
                .push(format!("[ASYNC_P2] {}: {}", v.pattern, v.fix)),
        }
    }
    None
}

/// database-ops — `P0Block` hard-blocks; P1/P2 advise.
pub(super) fn database_ops(ctx: &WriteContext<'_>, acc: &mut Acc) -> Option<String> {
    use kavach_patterns::database_ops_guard::DbOpsSeverity::{P0Block, P1Advisory, P2Warning};
    for v in &kavach_patterns::database_ops_guard::detect(ctx.file_path, ctx.content) {
        match v.severity {
            P0Block => return Some(format!("[DB_OPS_P0/{}] {}", v.pattern, v.fix)),
            P1Advisory => acc
                .p1_advisories
                .push(format!("[DB_OPS_P1] {}: {}", v.pattern, v.fix)),
            P2Warning => acc
                .p1_advisories
                .push(format!("[DB_OPS_P2] {}: {}", v.pattern, v.fix)),
        }
    }
    None
}

/// PII — `P0Block` hard-blocks; P1 advises.
pub(super) fn pii(ctx: &WriteContext<'_>, acc: &mut Acc) -> Option<String> {
    use kavach_patterns::pii_data_guard::PiiSeverity::{P0Block, P1Advisory};
    for v in &kavach_patterns::pii_data_guard::detect(ctx.file_path, ctx.content) {
        match v.severity {
            P0Block => return Some(format!("[PII_P0/{}] {}", v.pattern, v.fix)),
            P1Advisory => acc
                .p1_advisories
                .push(format!("[PII_P1] {}: {}", v.pattern, v.fix)),
        }
    }
    None
}

/// migration-safety — `P0Block` hard-blocks; P1 advises.
pub(super) fn migration(ctx: &WriteContext<'_>, acc: &mut Acc) -> Option<String> {
    use kavach_patterns::migration_safety_guard::MigSeverity::{P0Block, P1Advisory};
    for v in &kavach_patterns::migration_safety_guard::detect(ctx.file_path, ctx.content) {
        match v.severity {
            P0Block => return Some(format!("[MIGRATION_P0/{}] {}", v.pattern, v.fix)),
            P1Advisory => acc
                .p1_advisories
                .push(format!("[MIGRATION_P1] {}: {}", v.pattern, v.fix)),
        }
    }
    None
}

/// webhook-signature — `P0Block` hard-blocks; P1 advises.
pub(super) fn webhook(ctx: &WriteContext<'_>, acc: &mut Acc) -> Option<String> {
    use kavach_patterns::webhook_signature_guard::WhSeverity::{P0Block, P1Advisory};
    for v in &kavach_patterns::webhook_signature_guard::detect(ctx.file_path, ctx.content) {
        match v.severity {
            P0Block => return Some(format!("[WEBHOOK_P0/{}] {}", v.pattern, v.fix)),
            P1Advisory => acc
                .p1_advisories
                .push(format!("[WEBHOOK_P1] {}: {}", v.pattern, v.fix)),
        }
    }
    None
}

/// api-management — `P0Block` hard-blocks; P1/P2 advise.
pub(super) fn api_management(ctx: &WriteContext<'_>, acc: &mut Acc) -> Option<String> {
    use kavach_patterns::api_management_guard::ApiSeverity::{P0Block, P1Advisory, P2Warning};
    for v in &kavach_patterns::api_management_guard::detect(ctx.file_path, ctx.content) {
        match v.severity {
            P0Block => return Some(format!("[API_MGMT_P0/{}] {}", v.pattern, v.fix)),
            P1Advisory => acc
                .p1_advisories
                .push(format!("[API_MGMT_P1] {}: {}", v.pattern, v.fix)),
            P2Warning => acc
                .p1_advisories
                .push(format!("[API_MGMT_P2] {}: {}", v.pattern, v.fix)),
            _ => {}
        }
    }
    None
}

// split: intentional - cohesive gate inspection RPC group (inspect_bash + scan_write)
// JSON-RPC method handlers exposing kavach-patterns guards over the existing socket.
// Wraps destructive_cli_guard::inspect and the pre-write guard aggregator so any
// JSON-RPC client (hooks, IDE plugins, sibling agents) can call gates without shelling
// out to the CLI. Pure wrapper layer — no parallel transport, no new crate.
// SOURCE: https://docs.rs/jsonrpsee/latest/jsonrpsee/struct.RpcModule.html
mod severity_str;

use jsonrpsee::types::ErrorObjectOwned;
use kavach_patterns::destructive_cli_guard;
use serde::{Deserialize, Serialize};

use crate::state::AppState;

/// Cap on file content scanned per call (10 MiB). Shields the daemon from `DoS` via
/// arbitrarily large payloads from untrusted JSON-RPC peers.
const MAX_SCAN_CONTENT_BYTES: usize = 10 * 1024 * 1024;

fn invalid(msg: impl Into<String>) -> ErrorObjectOwned {
    ErrorObjectOwned::owned(-32602, msg.into(), None::<()>)
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct InspectBashParams {
    pub command: String,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct InspectBashResult {
    pub hit: bool,
    pub severity: Option<&'static str>,
    pub category: Option<&'static str>,
    pub pattern: Option<&'static str>,
    pub fix: Option<&'static str>,
    pub canonical: Option<String>,
}

/// Inspect a bash command for destructive patterns.
///
/// # Errors
///
/// This function does not return errors in the current implementation; all destructive
/// checks are performed synchronously and return results in `InspectBashResult`.
#[expect(
    clippy::needless_pass_by_value,
    reason = "RPC handler owns the deserialized params for the duration of the call"
)]
pub fn inspect_bash(
    _state: &AppState,
    params: InspectBashParams,
) -> Result<InspectBashResult, ErrorObjectOwned> {
    Ok(match destructive_cli_guard::inspect(&params.command) {
        Some(hit) => InspectBashResult {
            hit: true,
            severity: Some(hit.severity.as_str()),
            category: Some(hit.category.as_str()),
            pattern: Some(hit.pattern),
            fix: Some(hit.fix),
            canonical: Some(hit.canonical),
        },
        None => InspectBashResult {
            hit: false,
            severity: None,
            category: None,
            pattern: None,
            fix: None,
            canonical: None,
        },
    })
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct ScanWriteParams {
    pub file_path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct ScanFinding {
    pub gate: &'static str,
    pub severity: &'static str,
    pub pattern: &'static str,
    pub fix: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct ScanWriteResult {
    pub findings: Vec<ScanFinding>,
    pub p0_count: usize,
    pub advisory_count: usize,
}

/// Scan file content for pre-write safety violations across multiple guard patterns.
///
/// # Errors
///
/// Returns an error if the content length exceeds `MAX_SCAN_CONTENT_BYTES` (10 MiB).
#[expect(
    clippy::needless_pass_by_value,
    reason = "RPC handler owns the deserialized params for the duration of the call"
)]
pub fn scan_write(
    _state: &AppState,
    params: ScanWriteParams,
) -> Result<ScanWriteResult, ErrorObjectOwned> {
    if params.content.len() > MAX_SCAN_CONTENT_BYTES {
        return Err(invalid(format!(
            "content exceeds {MAX_SCAN_CONTENT_BYTES} bytes"
        )));
    }
    let path = params.file_path.as_str();
    let content = params.content.as_str();
    let mut findings = Vec::new();
    let mut push = |gate, severity, pattern, fix| {
        findings.push(ScanFinding {
            gate,
            severity,
            pattern,
            fix,
        });
    };

    for v in kavach_patterns::database_ops_guard::detect(path, content) {
        push(
            "database_ops_guard",
            severity_str::db_ops(v.severity),
            v.pattern,
            v.fix,
        );
    }
    for v in kavach_patterns::pii_data_guard::detect(path, content) {
        push(
            "pii_data_guard",
            severity_str::pii(v.severity),
            v.pattern,
            v.fix,
        );
    }
    for v in kavach_patterns::migration_safety_guard::detect(path, content) {
        push(
            "migration_safety_guard",
            severity_str::mig(v.severity),
            v.pattern,
            v.fix,
        );
    }
    for v in kavach_patterns::webhook_signature_guard::detect(path, content) {
        push(
            "webhook_signature_guard",
            severity_str::wh(v.severity),
            v.pattern,
            v.fix,
        );
    }
    for v in kavach_patterns::observability_guard::detect(path, content) {
        push(
            "observability_guard",
            severity_str::obs(v.severity),
            v.pattern,
            v.fix,
        );
    }
    for v in kavach_patterns::finops_guard::detect(path, content) {
        push(
            "finops_guard",
            severity_str::finops(v.severity),
            v.pattern,
            v.fix,
        );
    }
    for v in kavach_patterns::solid_guard::detect(path, content) {
        push(
            "solid_guard",
            severity_str::solid(v.severity),
            v.pattern,
            v.fix,
        );
    }
    for v in kavach_patterns::dsa_guard::detect(path, content) {
        push("dsa_guard", severity_str::dsa(v.severity), v.pattern, v.fix);
    }
    for v in kavach_patterns::axum_guard::detect(path, content) {
        push(
            "axum_guard",
            severity_str::axum(v.severity),
            v.pattern,
            v.fix,
        );
    }

    let p0_count = findings.iter().filter(|f| f.severity == "P0Block").count();
    let advisory_count = findings.len().saturating_sub(p0_count);
    Ok(ScanWriteResult {
        findings,
        p0_count,
        advisory_count,
    })
}

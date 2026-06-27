//! Vendor-native gate-failure policy.
//!
//! Two distinct failure modes, two policies:
//! - UNREADABLE input (gate never ran): Cursor fails OPEN so a parse glitch
//!   never wedges the IDE; Codex/Claude Code fail CLOSED (exit 2).
//! - Gate RAN but errored (RPC/DB outage): an ENFORCEMENT gate fails CLOSED on
//!   EVERY harness — including Cursor — because a backing-store outage must not
//!   silently allow a destructive op or break the stop loop. Observational
//!   gates fail OPEN (their block is meaningless).
use kavach_hook::Vendor;
use kavach_types::HookResponse;
/// `true` iff `gate_name` ENFORCES (its block actually prevents an action), so a
/// run-time failure inside it must fail CLOSED. The observational gates
/// (post-*, session/notification/etc.) cannot meaningfully block — a "block"
/// there is noise — so they fail OPEN (silent) on error regardless of vendor.
fn is_enforcement_gate(gate_name: &str) -> bool {
    matches!(gate_name, "pre-tool" | "pre-write" | "stop")
}
/// Emit a vendor-native FAILURE for an UNREADABLE payload and return its exit
/// code. The gate never ran — the JSON could not be parsed.
///
/// Cursor's native model is fail-OPEN, so a decode failure must let the action
/// through (a blocked-on-error gate wedges the editor — the original Cursor bug).
/// Codex and Claude Code fail CLOSED with the reason on stderr. This policy is
/// scoped to unreadable INPUT only; a gate that RAN and hit a backing-store
/// outage goes through [`fail_gate_error`], which fails closed even for Cursor.
#[expect(
    clippy::print_stderr,
    reason = "gate diagnostic path; no tracing subscriber in the hook binary"
)]
pub(super) fn fail_unreadable(vendor: Vendor, gate_name: &str, msg: &str) -> i32 {
    let resp = HookResponse::new_block(&format!(
        "kavach gate '{gate_name}': unreadable hook input ({msg})"
    ));
    if vendor == Vendor::Cursor {
        eprintln!("kavach gate '{gate_name}': unreadable input — Cursor fail-OPEN (allow)");
        return kavach_hook::output_native(vendor, &HookResponse::new_approve("fail-open"));
    }
    eprintln!(
        "kavach gate '{gate_name}': unreadable input ({msg}) — failing CLOSED (exit 2). \
         Anomalous stdin must not bypass the gate."
    );
    kavach_hook::output_native(vendor, &resp).max(2)
}
/// Emit a vendor-native FAILURE for a gate that RAN but errored — the canonical
/// RPC/DB-down path (`run_gate` returned `Err`). Returns its exit code.
///
/// Unlike [`fail_unreadable`], this fails CLOSED for an ENFORCEMENT gate
/// (pre-tool / pre-write / stop) on EVERY harness — INCLUDING Cursor. A
/// backing-store outage must not silently let a destructive op through or break
/// the stop loop; fail-OPEN there is the very safety hole A-3 closes.
/// Observational gates fail OPEN (silent, exit 0) since their block is
/// meaningless. The native renderer turns the block body into each harness's
/// contract (Cursor's loop-preserving `{continue:false,permission:deny}`,
/// Codex's exit 2, Claude Code's body+exit-2).
#[expect(
    clippy::print_stderr,
    reason = "gate diagnostic path; no tracing subscriber in the hook binary"
)]
pub(super) fn fail_gate_error(vendor: Vendor, gate_name: &str, msg: &str) -> i32 {
    if !is_enforcement_gate(gate_name) {
        eprintln!(
            "kavach gate '{gate_name}': error ({msg}) — observational gate, fail-OPEN (allow)"
        );
        return kavach_hook::output_native(vendor, &HookResponse::new_approve("fail-open"));
    }
    eprintln!(
        "kavach gate '{gate_name}': error ({msg}) — backing store down, failing CLOSED. \
         An enforcement gate must not bypass on outage (fail-closed, all vendors)."
    );
    let resp = HookResponse::new_block(&format!(
        "kavach gate '{gate_name}' could not reach its backing store ({msg}). \
         Failing CLOSED: resolve the kavach RPC/DB outage, then retry."
    ));
    kavach_hook::output_native(vendor, &resp).max(2)
}
#[cfg(test)]
#[path = "fail_test.rs"]
#[path = "fail_test.rs"]
mod tests;

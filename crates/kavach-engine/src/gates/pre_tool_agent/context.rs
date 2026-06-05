//! Context-injection builder: agent task + contract summary + guardrail lines.
use super::contract::AgentContract;

/// Build the `[AGENT_*]` context lines for a spawn (description + contract).
pub(super) fn build_agent_context(
    description: &str,
    contract: Option<&AgentContract>,
) -> Vec<String> {
    let mut ctx = Vec::new();
    if !description.is_empty() {
        ctx.push(format!("[AGENT_TASK] {description}"));
    }
    let Some(c) = contract else {
        return ctx;
    };
    ctx.push(format!(
        "[AGENT_CONTRACT] type={} read_only={} ttl={}s tools={}",
        c.name,
        c.read_only,
        c.ttl_seconds,
        c.allowed_tools.join(",")
    ));
    for g in c.guardrails {
        if let Some(line) = guardrail_line(g) {
            ctx.push(line.to_owned());
        }
    }
    ctx
}

/// Map a guardrail key to its injected advisory line.
fn guardrail_line(guardrail: &str) -> Option<&'static str> {
    match guardrail {
        "rca-required" => Some("[GUARDRAIL:RCA] Emit [RCA] before Write/Edit."),
        "blast-radius-check" => Some("[GUARDRAIL:RADIUS] Scan blast_radius."),
        "read-only" => Some("[GUARDRAIL:READ_ONLY] No mutations."),
        "no-execute" => Some("[GUARDRAIL:NO_EXEC] No Bash."),
        "no-secrets" => Some("[GUARDRAIL:NO_SECRETS] No credentials in output."),
        _ => None,
    }
}

//! Context-injection builder: agent task + contract + `BrainOS` + return-contract.
use super::contract::AgentContract;

/// `BrainOS` spawn context threaded from the session at gate time.
pub(super) struct BrainContext<'a> {
    pub project: &'a str,
    pub phase: &'a str,
    /// Smallest doer model for the invoking harness (haiku / composer-2.5 / …).
    pub doer_model: &'a str,
}

/// Build the `[AGENT_*]` context lines for a spawn: task, contract, `BrainOS`
/// state, and the return-contract that makes the subagent self-persist.
pub(super) fn build_agent_context(
    description: &str,
    contract: Option<&AgentContract>,
    brain: &BrainContext<'_>,
) -> Vec<String> {
    let mut ctx = Vec::new();
    if !description.is_empty() {
        ctx.push(format!("[AGENT_TASK] {description}"));
    }
    if !brain.project.is_empty() {
        ctx.push(format!(
            "[AGENT_BRAINOS] project={} phase={} — read the kavach DB, never re-derive \
             what a row already holds",
            brain.project, brain.phase
        ));
    }
    if !brain.doer_model.is_empty() {
        ctx.push(format!(
            "[AGENT_SPAWN] spawn a DYNAMIC subagent (inline task, not a predefined \
             .claude/agents type) on the smallest doer model: {} — orchestrate strong, \
             execute cheap.",
            brain.doer_model
        ));
    }
    ctx.push(
        "[AGENT_RESEARCH] invoke the precise skill for the task; WebSearch the CURRENT \
         authoritative source before deciding — NEVER trust training weights, they are stale."
            .to_owned(),
    );
    if let Some(c) = contract {
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
        ctx.push(return_contract(c.read_only));
    }
    ctx
}

/// The return-contract: a writer persists its finding to Kavach the same turn;
/// a read-only agent returns a structured result for the orchestrator to persist.
fn return_contract(read_only: bool) -> String {
    if read_only {
        "[AGENT_RETURN_CONTRACT] return a structured result (findings + file:line \
         evidence), not prose — the orchestrator persists it to the kavach DB."
            .to_owned()
    } else {
        "[AGENT_RETURN_CONTRACT] persist every settled decision to the kavach DB the \
         SAME turn (choice + source + one-line why); a finding not written is lost. \
         Close by 3-witness, never hand work back."
            .to_owned()
    }
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

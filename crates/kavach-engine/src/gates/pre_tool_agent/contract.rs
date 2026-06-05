//! Agent security contracts: allowed tools, guardrails, read-only flag, TTL.
//!
//! Hermes pattern: typed contracts enable self-improving skill extraction.
//! SOURCE: code.claude.com/docs/en/agent-sdk/hooks.

/// Per-agent-type security contract.
pub(crate) struct AgentContract {
    pub name: &'static str,
    pub allowed_tools: &'static [&'static str],
    pub guardrails: &'static [&'static str],
    pub read_only: bool,
    pub ttl_seconds: u64,
}

/// Registry of known agent types with their security contracts.
static AGENT_CONTRACTS: &[AgentContract] = &[
    AgentContract {
        name: "research-director",
        allowed_tools: &["WebSearch", "WebFetch", "Read", "Glob", "Grep"],
        guardrails: &["read-only", "no-secrets"],
        read_only: true,
        ttl_seconds: 300,
    },
    AgentContract {
        name: "code-reviewer",
        allowed_tools: &["Read", "Glob", "Grep", "LSP"],
        guardrails: &["read-only", "no-execute"],
        read_only: true,
        ttl_seconds: 180,
    },
    AgentContract {
        name: "Explore",
        allowed_tools: &["Read", "Glob", "Grep", "Bash"],
        guardrails: &["read-only"],
        read_only: true,
        ttl_seconds: 120,
    },
    AgentContract {
        name: "backend-engineer",
        allowed_tools: &["Read", "Write", "Edit", "Bash", "Grep", "Glob", "WebSearch"],
        guardrails: &["rca-required", "blast-radius-check"],
        read_only: false,
        ttl_seconds: 600,
    },
    AgentContract {
        name: "spec-author",
        allowed_tools: &["Read", "Glob", "Grep", "WebSearch", "WebFetch", "Bash"],
        guardrails: &["read-only"],
        read_only: true,
        ttl_seconds: 300,
    },
    AgentContract {
        name: "context-curator",
        allowed_tools: &["Read", "Glob", "Grep", "Bash"],
        guardrails: &["read-only"],
        read_only: true,
        ttl_seconds: 180,
    },
];

/// Look up the contract for an agent type, if registered.
pub(crate) fn get_contract(agent_type: &str) -> Option<&'static AgentContract> {
    AGENT_CONTRACTS.iter().find(|c| c.name == agent_type)
}

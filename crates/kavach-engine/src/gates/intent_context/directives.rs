//! Intent-keyed context directives: forbidden phrases, memory-DB / verify-existing
//! reminders, the Root-Cause protocol, and the agent/skill dispatch matrix.

/// Append forbidden phrase warnings to context.
pub(crate) fn append_forbidden(context: &mut String, forbidden: &[String]) {
    if forbidden.is_empty() {
        return;
    }
    context.push_str("\n[FORBIDDEN_PHRASES]\n");
    for phrase in forbidden {
        context.push_str("  - ");
        context.push_str(phrase);
        context.push('\n');
    }
}

/// Append memory DB reminder for memory-type intents.
pub(crate) fn append_memory_db(context: &mut String, intent_type: &str) {
    if intent_type == "memory" {
        context.push_str("\n[MEMORY_DB] Use kavach db write — NOT MEMORY.md files\n");
    }
}

/// Append verify-existing reminder for implement-type intents.
pub(crate) fn append_verify_existing(context: &mut String, intent_type: &str) {
    if intent_type == "implement" || intent_type == "debug" {
        context
            .push_str("\n[VERIFY_EXISTING] Read existing routes/handlers/models before planning\n");
    }
}

/// Append Root-Cause Analysis protocol for debug/fix/refactor intents.
/// Enforces offensive deep investigation — forbids surface patches.
/// SOURCES: five-whys RCA + `RCAEval` (FSE'26) + sjmyuan/prompts.
// ARCH: prompt-cache-tier-split | PROBLEM_CLASS: cache
// TIME: O(1) | SPACE: O(1) | YEAR: 2026 | SEARCHED: 2026-04
pub(crate) fn append_root_cause_protocol(context: &mut String, intent_type: &str) {
    if intent_type != "debug" && intent_type != "refactor" && intent_type != "implement" {
        return;
    }
    context.push_str(
        "\n[ROOT_CAUSE_PROTOCOL] See CLAUDE.md §1. Output [RCA] block before Write/Edit:\n\
         symptom · repro(file:line) · why1..why5(evidence) · root_cause · class · \
         blast_radius · research(URL) · fix_strategy. Gate BLOCKS without it.\n",
    );
}

/// Append agent + skill dispatch directives by intent type.
/// Routing matrix (validated 2026-04 via Anthropic subagent docs):
///   debug → ceo + bug-bounty · refactor → aegis-guardian + rust ·
///   implement → writing-plans · general → research-director.
/// SOURCES: code.claude.com/docs/en/sub-agents · claudefa.st sub-agent best practices.
pub(crate) fn append_agent_dispatch(context: &mut String, intent_type: &str) {
    let directive = match intent_type {
        "debug" => {
            "\n[INVOKE_AGENT: ceo] [INVOKE_SKILL: bug-bounty]\n\
             Spawn ceo NOW; ceo routes to specialist. Skill bug-bounty owns the 5-why hunt.\n"
        }
        "refactor" => {
            "\n[INVOKE_AGENT: aegis-guardian] [INVOKE_SKILL: rust]\n\
             aegis-guardian verifies invariants; engineer applies fix. Skill rust owns ownership/lifetime moves.\n"
        }
        "implement" => {
            "\n[INVOKE_SKILL: writing-plans]\n\
             Plan first. iteration-start before edit. iteration-done before next file.\n"
        }
        "general" => {
            "\n[INVOKE_AGENT: research-director]\n\
             research-director runs read-only investigation; engineers act on findings.\n"
        }
        _ => return,
    };
    context.push_str(directive);
}

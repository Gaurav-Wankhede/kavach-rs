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
// REJECTED: [{"name":"keep-30-line-inline","reason":"700 tokens/turn uncached"},{"name":"remove-injection","reason":"loses [RCA] gate enforcement"}]
// TIME: O(1) | SPACE: O(1) | YEAR: 2026 | SEARCHED: 2026-04
// BENCHMARK: https://www.dbreunig.com/2026/04/04/how-claude-code-builds-a-system-prompt.html
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

/// Append agent dispatch directives, dynamically ranked when possible, else
/// the intent-keyed default table (2026 hybrid routing best practice).
/// SOURCE: <https://www.merge.dev/blog/llm-routing> · <https://arxiv.org/pdf/2511.02200>.
pub(crate) fn append_agent_dispatch(
    context: &mut String,
    intent_type: &str,
    prompt: &str,
    research_topic: &str,
) {
    if try_dynamic_dispatch(context, prompt, research_topic) {
        return;
    }
    append_static_dispatch(context, intent_type);
}

/// Minimum distinct prompt-word overlap for a ranked agent to be trusted over
/// the static default. Below this the prompt is too generic — defer to the table.
const DYNAMIC_DISPATCH_FLOOR: usize = 2;

/// Try to inject a DB/research-ranked agent directive. Returns `true` when a
/// confident match was injected, `false` to fall through to the static table.
fn try_dynamic_dispatch(context: &mut String, prompt: &str, research_topic: &str) -> bool {
    let Some(loader) = kavach_chain::loader::global_loader() else {
        return false;
    };
    // Enrich the ranking query with the live research topic so a researched
    // turn steers the agent choice (internet-first feeds dispatch).
    let query = if research_topic.is_empty() {
        prompt.to_owned()
    } else {
        format!("{prompt} {research_topic}")
    };
    let ranked = loader.rank_agents_for_prompt(&query, 1);
    let Some((agent, score)) = ranked.into_iter().next() else {
        return false;
    };
    if score < DYNAMIC_DISPATCH_FLOOR {
        return false;
    }
    context.push_str("\n[INVOKE_AGENT: ");
    context.push_str(&agent.name);
    context.push_str("] (dynamic, score=");
    context.push_str(&score.to_string());
    context.push_str(")\n");
    context.push_str(&agent.description);
    context.push('\n');
    true
}

/// Intent-keyed default table — the hybrid fallback when ranking is inconclusive.
fn append_static_dispatch(context: &mut String, intent_type: &str) {
    let directive = match intent_type {
        "debug" => {
            "\n[INVOKE_AGENT: ceo] [INVOKE_SKILL: bug-bounty]\n\
             Spawn ceo NOW; ceo routes to specialist. Skill bug-bounty owns the 5-why hunt.\n"
        }
        "refactor" => {
            "\n[INVOKE_AGENT: aegis-guardian] [INVOKE_SKILL: rust]\n\
             aegis-guardian verifies invariants; engineer applies fix. Skill rust owns holdership/lifetime moves.\n"
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

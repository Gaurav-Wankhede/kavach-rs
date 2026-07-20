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

use crate::gates::directive_cache::dyn_directive;

/// Compiled fan-out law fallback — mirrors gate_config `*/harness.fanout_law`.
/// Keeps the migration copy-first rule inline so a dispatched agent never rewrites
/// a port from scratch. SOURCE: decision.harness.fanout-to-cheap-tier.
const FANOUT_LAW_FALLBACK: &str = "If you dispatch an agent, use the cheap tier \
    (claude-haiku-4-5). MIGRATION RULE: FIRST copy the source file with cp, THEN apply only the \
    minimal framework/language-specific edits; NEVER rewrite from scratch. SOURCE: \
    anthropic.com/engineering/multi-agent-research-system.";

/// Append the `[FANOUT_LAW]` directive after a dispatched agent, so the orchestrator
/// always delegates the labor to the cheap tier. Skill-only routes never call this.
fn append_fanout_law(context: &mut String) {
    context.push_str("[FANOUT_LAW] ");
    context.push_str(&dyn_directive("harness.fanout-law", FANOUT_LAW_FALLBACK));
    context.push('\n');
}

/// Append memory DB reminder for memory-type intents. The `[MEMORY_DB]` tag is a
/// fixed contract; the imperative after it is research-cached (fail-soft literal).
pub(crate) fn append_memory_db(context: &mut String, intent_type: &str) {
    if intent_type == "memory" {
        context.push_str("\n[MEMORY_DB] ");
        context.push_str(&dyn_directive(
            "intent.memory-db",
            "Use kavach db write — NOT MEMORY.md files",
        ));
        context.push('\n');
    }
}

/// Append verify-existing reminder for implement-type intents.
pub(crate) fn append_verify_existing(context: &mut String, intent_type: &str) {
    if intent_type == "implement" || intent_type == "debug" {
        context.push_str("\n[VERIFY_EXISTING] ");
        context.push_str(&dyn_directive(
            "intent.verify-existing",
            "Read existing routes/handlers/models before planning",
        ));
        context.push('\n');
    }
}

/// Append Root-Cause Analysis protocol for debug/fix/refactor intents.
/// SOURCE: `decision.engine.rca_protocol_inject`
pub(crate) fn append_root_cause_protocol(context: &mut String, intent_type: &str) {
    if intent_type != "debug" && intent_type != "refactor" && intent_type != "implement" {
        return;
    }
    context.push_str("\n[ROOT_CAUSE_PROTOCOL] ");
    context.push_str(&dyn_directive(
        "intent.root-cause-protocol",
        "Before fix-Write emit [RCA]: symptom@file:line → why-chain→root_cause · \
         class+blast · fix · cite:URL. Fix cause ≠ symptom.",
    ));
    context.push('\n');
}

/// Append migration/porting law for implement/refactor intents when the prompt
/// signals a framework or language migration. Directs the agent to copy source
/// files first and make only minimal edits, avoiding context-rot rewrites.
pub(crate) fn append_migration_law(context: &mut String, intent_type: &str, prompt: &str) {
    if intent_type != "implement" && intent_type != "refactor" && intent_type != "general" {
        return;
    }
    let lower = prompt.to_lowercase();
    let is_migration = lower.contains("migrat")
        || lower.contains("port ")
        || lower.contains("porting")
        || lower.contains("convert")
        || lower.contains("conversion")
        || lower.contains("translate")
        || lower.contains("rewrite")
        || lower.contains("upgrade to")
        || lower.contains("move to")
        || lower.contains("switch to");
    if !is_migration {
        return;
    }
    context.push_str("\n[MIGRATION_LAW] ");
    context.push_str(&dyn_directive(
        "intent.migration-law",
        "Migration / port / conversion: copy the source file with cp FIRST, then apply ONLY \
         the minimal framework/language-specific edits needed to make it run in the target \
         stack. NEVER rewrite the file from scratch. Preserve original logic, algorithms, \
         variable names, and comments unless the target framework or language forces a change.",
    ));
    context.push('\n');
}

/// Check if prompt contains diagram-related keywords.
fn has_diagram_keyword(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    lower.contains("architecture")
        || lower.contains("diagram")
        || lower.contains("design")
        || lower.contains("flow")
        || lower.contains("structure")
        || lower.contains("component")
        || lower.contains("sequence")
        || lower.contains("state machine")
        || lower.contains("lld")
        || lower.contains("hld")
}

/// Append the diagram-first standing law for plan/design/implement intents.
/// Advisory tier (steers, never blocks). SOURCE: decision.harness.sdlc-nano-agents-global.
pub(crate) fn append_diagram_first(context: &mut String, intent_type: &str, prompt: &str) {
    let intent_matches = intent_type == "implement"
        || intent_type == "refactor"
        || intent_type == "general"
        || intent_type == "plan"
        || intent_type == "design"
        || intent_type == "architecture";
    if !intent_matches && !has_diagram_keyword(prompt) {
        return;
    }
    context.push_str("\n[DIAGRAM_FIRST] ");
    context.push_str(&dyn_directive(
        "intent.diagram-first",
        "If this turn proposes architecture or an LLD, FIRST emit a temp HTML file with a \
         validated Mermaid diagram and open it for the user BEFORE deciding. Use ESM import, \
         no SRI, and run `just mermaid-check <file>` first.",
    ));
    context.push('\n');
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
    append_fanout_law(context);
    true
}

/// Intent-keyed default table — the hybrid fallback when ranking is inconclusive.
/// The `[INVOKE_AGENT/SKILL: …]` routing tags stay literal (parsed downstream);
/// only the trailing imperative prose is research-cached, so the routing target
/// is deterministic while its rationale stays current.
fn append_static_dispatch(context: &mut String, intent_type: &str) {
    let (tags, key, prose) = match intent_type {
        "debug" => (
            "\n[INVOKE_AGENT: ceo] [INVOKE_SKILL: bug-bounty]\n",
            "dispatch.debug",
            "Spawn ceo NOW; ceo routes to specialist. Skill bug-bounty owns the 5-why hunt.",
        ),
        "refactor" => (
            "\n[INVOKE_AGENT: aegis-guardian] [INVOKE_SKILL: rust]\n",
            "dispatch.refactor",
            "aegis-guardian verifies invariants; engineer applies fix. Skill rust owns holdership/lifetime moves.",
        ),
        "implement" => (
            "\n[INVOKE_SKILL: writing-plans]\n",
            "dispatch.implement",
            "Plan first. iteration-start before edit. iteration-done before next file.",
        ),
        "general" => (
            "\n[INVOKE_AGENT: research-director]\n",
            "dispatch.general",
            "research-director runs read-only investigation; engineers act on findings.",
        ),
        _ => return,
    };
    context.push_str(tags);
    context.push_str(&dyn_directive(key, prose));
    context.push('\n');
    if tags.contains("INVOKE_AGENT") {
        append_fanout_law(context);
    }
}

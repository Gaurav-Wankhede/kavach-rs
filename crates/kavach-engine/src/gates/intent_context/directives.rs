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
        "See CLAUDE.md §1. Output [RCA] block before Write/Edit:\n\
         symptom · repro(file:line) · why1..why5(evidence) · root_cause · class · \
         blast_radius · research(URL) · fix_strategy. Gate BLOCKS without it.",
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

/// Append the diagram-first standing law for plan/design/implement intents: a
/// turn that proposes architecture or an LLD must emit a temp HTML+Mermaid view
/// (Mermaid ESM import + mermaid.run, NO SRI) and surface it to the user BEFORE deciding, so the
/// structure is reviewable before any code. Advisory tier (steers, never blocks).
/// SOURCE: decision.harness.sdlc-nano-agents-global · diagram-first law.
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
        "When this turn proposes architecture or a low-level design, FIRST write a \
         temp HTML file with a Mermaid diagram (LLD: every component + typed edge + \
         node→file:symbol map) and open it for the user, BEFORE ExitPlanMode / before \
         deciding. Spawn the architect-lld agent to emit the Mermaid. Load Mermaid via \
         ESM `import()` (jsdelivr .esm.min.mjs with an unpkg fallback), call \
         `mermaid.initialize({startOnLoad:false, securityLevel:'loose'})` then \
         `await mermaid.run()`, and show a visible warning if every CDN is blocked — \
         NO SRI integrity= tag (a guessed/stale hash makes the browser refuse the \
         script and the diagram silently renders as raw text). The diagram is the \
         review surface — the user decides from it, not from prose.",
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
}

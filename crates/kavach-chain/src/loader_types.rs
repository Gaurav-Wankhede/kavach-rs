use std::collections::HashSet;

//   {"name":"Vec<String> + .contains","reason":"O(N) scan; HashSet idiomatic for membership"},
//   {"name":"phf::Set perfect-hash","reason":"compile-time hash overhead unjustified for runtime-loaded data"},
//   {"name":"&'static str slice","reason":"agent files load at runtime; compile-time impossible"}
// ]
// TIME: O(1) average is_research_class check
// SPACE: O(N) per agent where N≤20
// YEAR: 2026 | SEARCHED: 2026-05
// SOURCE: ~/.claude/agents/*.md frontmatter shape verified via bat -p
//   (research-director, research-evolutionist, pattern-extractor) — fields
//   `tools:` and `disallowedTools:` are comma-separated YAML scalars between
//   triple-dash markers. See decision:rca.loader_types_research_class.
// SOURCE: https://yaml.org/spec/1.2.2/#scalars-and-tags
// SOURCE: https://github.com/anthropics/claude-code/issues/31292
//   (disallowedTools=[Write,Edit] is bypassable via Bash sed/echo) — therefore
//   is_research_class() requires BOTH the disallow and the absence-from-allow
//   path to be safe. Defense-in-depth, not single-check.
#[derive(Debug, Clone, Default)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundaries"
)]
pub struct AgentDef {
    pub name: String,
    pub description: String,
    pub model: String,
    pub skills: Vec<String>,
    pub priority: i32,
    /// Tools the agent is allowed to invoke (frontmatter `tools:`).
    /// If non-empty, this is an exclusive allowlist — agent can ONLY use these.
    pub tools: HashSet<String>,
    /// Tools the agent is explicitly forbidden (frontmatter `disallowedTools:`).
    pub disallowed_tools: HashSet<String>,
    /// Intent-fit tags from frontmatter `capabilities:` field. Each tag should
    /// match an `intent_type` from `intent_tree.rs` (implement | debug | security
    /// | deploy | refactor | memory | general). Empty = no declared fit;
    /// router falls back to description-overlap ranking.
    /// SOURCE: `decision:rca.intent_aware_capability_routing`
    /// SOURCE: <https://lib.rs/crates/gray_matter> (frontmatter parsing pattern)
    pub capabilities: HashSet<String>,
}

impl AgentDef {
    /// True iff frontmatter contract makes the agent provably read-only AND
    /// research-capable (research-class).
    ///
    /// SAFETY: Issue #31292 proved that `disallowedTools: [Write, Edit]` alone
    /// is bypassable via `Bash` (sed/echo/tee). The contract is only sound if:
    /// (a) `tools` is a non-empty allowlist that excludes ALL write paths
    ///     (Write, Edit, Bash, `NotebookEdit`), AND
    /// (b) at least one of `WebSearch` or `WebFetch` is in the allowlist.
    /// Falls back to `disallowed_tools` check only when `tools` is empty —
    /// in that case requires Write+Edit+Bash+NotebookEdit ALL disallowed.
    #[must_use]
    pub(crate) fn is_research_class(&self) -> bool {
        let write_paths = ["Write", "Edit", "Bash", "NotebookEdit"];
        let research_tools = ["WebSearch", "WebFetch"];

        let research_capable_in_tools = research_tools.iter().any(|t| self.tools.contains(*t));

        // Branch 1: explicit allowlist mode (tools is non-empty).
        // Empty `tools: []` falls through to branch 2 — an explicitly empty
        // allowlist means no tools allowed, which is a stricter form of
        // read-only and ALSO satisfies the no-write-paths invariant; but it
        // also disallows WebSearch/WebFetch, so research_capable_in_tools is
        // false → not research-class. Disallow-only mode handles the
        // intermediate case (no `tools:` declared, only `disallowedTools:`).
        if !self.tools.is_empty() {
            let no_write_in_allowlist = write_paths.iter().all(|t| !self.tools.contains(*t));
            return research_capable_in_tools && no_write_in_allowlist;
        }

        // Branch 2: disallow-only mode (no allowlist). Research-class iff
        // every write path is explicitly disallowed (defense-in-depth per
        // Issue #31292) AND at least one research tool is implicitly allowed
        // (i.e., NOT in disallowed_tools).
        let all_writes_blocked = write_paths
            .iter()
            .all(|t| self.disallowed_tools.contains(*t));
        let research_implicitly_allowed = research_tools
            .iter()
            .any(|t| !self.disallowed_tools.contains(*t));
        all_writes_blocked && research_implicitly_allowed
    }
}

#[derive(Debug, Clone)]
#[expect(
    clippy::exhaustive_structs,
    reason = "RPC DTO constructed at handler boundaries"
)]
pub struct SkillDef {
    pub name: String,
    pub description: String,
    pub triggers: Vec<String>,
    pub auto_invoke: bool,
    pub content: String,
}

pub(crate) fn extract_description(content: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("description:") {
            return rest.trim().to_owned();
        }
    }
    String::new()
}

pub(crate) fn extract_triggers(content: &str) -> Vec<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("triggers:") {
            return rest
                .split(',')
                .map(|t| t.trim().to_owned())
                .filter(|t| !t.is_empty())
                .collect();
        }
    }
    Vec::new()
}

/// Parse a YAML field by key prefix from frontmatter content.
/// Supports BOTH forms (verified across 42 agent files):
///   inline:  `tools: Read, Write, Edit`
///   inline-bracket: `tools: [Read, Write, Edit]`
///   block-list: `tools:\n  - Read\n  - Write\n  - Edit`
/// Returns empty `HashSet` on absence — safe-fail per `AgentDef::Default`.
fn extract_csv_set(content: &str, key_prefix: &str) -> HashSet<String> {
    let mut lines = content.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix(key_prefix) else {
            continue;
        };
        let rest_trimmed = rest.trim();
        if !rest_trimmed.is_empty() {
            // Inline or inline-bracket form
            return rest_trimmed
                .split(',')
                .map(|t| {
                    t.trim()
                        .trim_start_matches('[')
                        .trim_end_matches(']')
                        .trim()
                })
                .filter(|t| !t.is_empty())
                .map(ToOwned::to_owned)
                .collect();
        }
        // Block-list form: collect subsequent indented `- value` lines
        let mut out = HashSet::new();
        while let Some(peek) = lines.peek() {
            let pt = peek.trim_start();
            if let Some(item) = pt.strip_prefix("- ").or_else(|| pt.strip_prefix("-\t")) {
                let v = item.trim().trim_matches('"').trim_matches('\'');
                if !v.is_empty() {
                    out.insert(v.to_owned());
                }
                lines.next();
            } else if pt.is_empty() {
                lines.next();
            } else {
                break;
            }
        }
        return out;
    }
    HashSet::new()
}

/// Extract `tools:` allowlist from agent frontmatter.
/// SOURCE: <https://github.com/anthropics/claude-code/issues/6005> (allowlist semantics)
pub(crate) fn extract_tools(content: &str) -> HashSet<String> {
    extract_csv_set(content, "tools:")
}

/// Extract `disallowedTools:` denylist from agent frontmatter.
/// SOURCE: <https://github.com/anthropics/claude-code/issues/6005>
pub(crate) fn extract_disallowed_tools(content: &str) -> HashSet<String> {
    extract_csv_set(content, "disallowedTools:")
}

/// Extract `capabilities:` intent-fit tags from agent frontmatter.
/// Tags should match `intent_type` values from `intent_tree.rs`.
/// Returns empty set when absent — router treats this as wildcard fallback.
pub(crate) fn extract_capabilities(content: &str) -> HashSet<String> {
    extract_csv_set(content, "capabilities:")
}

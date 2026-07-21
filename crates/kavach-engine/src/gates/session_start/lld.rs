//! `[KAVACH_LLD]` self-architecture awareness — the cure for kavach amnesia.
//!
//! Injects a compact Mermaid low-level-design map (hook lifecycle + crate layers +
//! data flow) at SessionStart so an agent working ON kavach opens with the system's
//! own model in hand, instead of re-deriving it from `--help`/`rg` every turn. One
//! diagram beats a paragraph. The full human-readable version lives at
//! `docs/architecture/kavach-lld.html`; this is the token-tight injected sibling.
//!
//! Scoped to the kavach project ONLY — every other project's SessionStart is
//! untouched, so this never spends another codebase's token budget on kavach's map.
/// Project slugs for which the kavach self-architecture is relevant. A non-kavach
/// project gets `None` (the block is omitted entirely).
fn is_kavach_project(project: &str) -> bool {
    let p = project.to_ascii_lowercase();
    p == "kavach-rs" || p == "kavach" || p.starts_with("kavach-")
}
/// The compact LLD awareness block — a single Mermaid flowchart of the governing
/// loop plus a one-line crate/CLI/store legend. Kept tight on purpose: the session
/// already carries the live board + ledger; this is the static self-model only.
const KAVACH_LLD: &str = "[KAVACH_LLD] kavach self-architecture (full HTML: docs/architecture/kavach-lld.html)\n\
```mermaid\n\
flowchart TD\n\
  TOOL[\"AI tool hook event\"] --> GATES[\"kavach gates (event)\"]\n\
  GATES --> ENG[\"kavach-engine: gate dispatch\"]\n\
  ENG --> SS[\"SessionStart: inject board, ledger, DECISION_MAP, KAVACH_LLD\"]\n\
  ENG --> PW[\"PreWrite/Bash: RCA, TDD, security guards -> allow/block/ask\"]\n\
  ENG --> STOP[\"Stop: 3-witness + dispatch next card\"]\n\
  ENG <-->|JSON-RPC single writer| RPC[\"kavach-rpc\"]\n\
  RPC <-->|read/write| DB[(\"SurrealDB: kanban, decision, anti_pattern, concepts\")]\n\
  ENG --> PAT[\"kavach-patterns: ~100 detectors\"]\n\
  DB -.decision overlay.-> ENG\n\
```\n\
LAYERS: cli/hook/web -> engine+patterns+advisor -> rpc+surreal+session+config -> types. \
21 hook events: lifecycle(session-start|session-end|pre-compact|stop|notification) · \
prompt(intent) · write/tool(pre-write|post-write|pre-tool|post-tool|post-tool-failure) · \
impl · subagent(start|stop) · permission(permission|permission-request) · \
vendor(message-display|task-completed|teammate-idle). \
CLI groups (~35 verbs): store(db get|write|kanban-close|context) · loop(loop|heal|loophole|goal|bg|team|pipeline|bulk) · \
awareness(think|ask|doctor|oversized|spec|schema|security) · \
lifecycle(phase|session|verify|verify-frontend|deploy|mistake) · ops(status|web|servers|gates|install|rules|toolbelt|tailwind-plus|todos|tasks). \
Source of truth for any flag: `kavach <cmd> --help`.\n";
/// `Some(block)` with the kavach LLD awareness for a kavach project, else `None`.
///
/// Fail-soft by construction: a non-kavach project (or empty slug) yields `None`,
/// so the block never appears where it is irrelevant. The content is a compiled
/// constant — no RPC, no DB, so it cannot fail or stall the hot SessionStart path.
#[must_use]
pub(super) fn lld_context(project: &str) -> Option<String> {
    if project.is_empty() || !is_kavach_project(project) {
        return None;
    }
    Some(KAVACH_LLD.to_owned())
}
#[cfg(test)]
#[path = "lld_test.rs"]
mod tests;

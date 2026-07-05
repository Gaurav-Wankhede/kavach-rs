//! The advisory block shown when a forbidden six-file markdown path is written —
//! maps each legacy doc to its kavach-db–native row equivalent.

/// Format the `[SIX_FILE_BLOCK]` advisory for a forbidden `path`.
pub(super) fn format_block(path: &str) -> String {
    format!(
        "[SIX_FILE_POLICY] Forbidden path: {path}\n\n\
         The Six-File Context methodology is kavach-db–native (CLAUDE.md §15).\n\
         Markdown spec/context files duplicate state already typed in kavach-db\n\
         and create drift between docs and code.\n\n\
         Use kavach-db rows instead:\n\
         - project-overview     → category=app_spec, key-prefix=spec.*\n\
         - architecture         → category=architecture, key-prefix=arch.*\n\
         - ui-context           → category=app_spec, key-prefix=ui.token.*\n\
         - code-standards       → table=gate_patterns\n\
         - ai-workflow-rules    → category=decision, key-prefix=workflow.rule.*\n\
         - progress-tracker     → table=kanban_cards (already typed)\n\
         - specs/NN-feature.md  → category=roadmap, key=roadmap.unit.NN.<slug>\n\n\
         Invoke Skill `six-file-context` for the write protocol, or Agent\n\
         `spec-author` to draft the rows. Then `kavach db write --category <cat>\n\
         --key <key> --content \"<spec>\"` -> retry."
    )
}

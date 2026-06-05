// ARCH: see kavach db get --category decision --key arch.decision.fourteen_prefix_const_table

use kavach_types::{AutoDraftSource, MissingPrefix};

#[must_use]
pub(crate) fn draft_block(missing: &MissingPrefix) -> String {
    match missing.auto_draftable {
        AutoDraftSource::HumanOnly => format!(
            "Prefix {}: routing to Agent `spec-author`\n  \
             Agent `spec-author` is read-only and will draft the {} template.\n  \
             Parent will write via: kavach db write --category app_spec --key {} --content \"<spec>\"",
            missing.point, missing.label, missing.key_prefix
        ),
        AutoDraftSource::CodebaseAst => format!(
            "Prefix {}: scan codebase AST\n  \
             Run: fd -e rs . src/ | xargs rg 'pub struct|pub enum' to list types.\n  \
             Then write the data model schema to: kavach db write --category architecture --key arch.data --content \"<schema>\"",
            missing.point
        ),
        AutoDraftSource::GitLog => format!(
            "Prefix {}: extract from git history\n  \
             Run: git log --oneline -30 to see recent changes.\n  \
             Synthesize a roadmap unit: kavach db write --category roadmap --key roadmap.unit.N.<slug> --content \"<unit>\"",
            missing.point
        ),
        AutoDraftSource::HandlerScan => format!(
            "Prefix {}: scan request handlers\n  \
             Find HTTP handlers (axum/actix routes) and document the API contracts.\n  \
             Write to: kavach db write --category architecture --key arch.api --content \"<contracts>\"",
            missing.point
        ),
        AutoDraftSource::TracingScan => format!(
            "Prefix {}: scan instrumentation\n  \
             List all tracing::info!, tracing::warn!, and metric calls in src/.\n  \
             Document observability plan: kavach db write --category architecture --key arch.obs --content \"<observability>\"",
            missing.point
        ),
        AutoDraftSource::RouteScan => format!(
            "Prefix {}: extract UI routes\n  \
             Document all UI/component paths and flows from router configuration.\n  \
             Write: kavach db write --category app_spec --key ui.flow --content \"<flows>\"",
            missing.point
        ),
        AutoDraftSource::TestScan => format!(
            "Prefix {}: list test scenarios\n  \
             Run: fd -e rs . src/ | xargs rg '#\\[test\\]' to find existing test cases.\n  \
             Document edge cases and user stories: kavach db write --category app_spec --key spec.story --content \"<stories>\"",
            missing.point
        ),
        // AutoDraftSource is #[non_exhaustive]: unknown upstream variant falls back
        // to the human-routed default rather than failing the draft.
        _ => format!(
            "Prefix {}: routing to Agent `spec-author` (unknown auto-draft source)",
            missing.point
        ),
    }
}

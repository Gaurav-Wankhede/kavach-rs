// ARCH: see kavach db get --category decision --key arch.decision.fourteen_prefix_const_table

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(super) fn run(action: crate::cli::SpecAction) -> i32 {
    match action {
        crate::cli::SpecAction::AutoDraft { prefix, project } => {
            handle_auto_draft(&prefix, &project)
        }
    }
}

fn handle_auto_draft(prefix: &str, project: &str) -> i32 {
    if let Some(required_prefix) = kavach_types::FOURTEEN_PREFIXES
        .iter()
        .find(|p| p.key_prefix == prefix || p.label.to_lowercase() == prefix.to_lowercase())
    {
        let write_hint = format!(
            "kavach db write --project {project} --category {} --new \
             --key {}.<slug> --title \"<title>\" --content \"<spec>\"",
            required_prefix.category.as_db_str(),
            required_prefix.key_prefix,
        );
        let scope_hint = format!(
            "(scoped to project '{project}'; existing rows: \
             kavach db query --project {project} --category {})",
            required_prefix.category.as_db_str(),
        );
        let header = format!(
            "[AUTO_DRAFT] project={project} point={} label={} key_prefix={}",
            required_prefix.point, required_prefix.label, required_prefix.key_prefix,
        );
        let msg = match required_prefix.auto_draftable {
            kavach_types::AutoDraftSource::HumanOnly => format!(
                "{header}\nsource: HumanOnly — route to Agent `spec-author`.\n{scope_hint}\nthen: {write_hint}"
            ),
            kavach_types::AutoDraftSource::CodebaseAst => format!(
                "{header}\nsource: CodebaseAst — scan struct/enum AST.\n\
                 run: fd -e rs . src/ | xargs rg 'pub struct|pub enum' > /tmp/{project}-codebase-ast.txt\n\
                 then: {write_hint}"
            ),
            kavach_types::AutoDraftSource::GitLog => format!(
                "{header}\nsource: GitLog — extract recent commits.\n\
                 run: git log --oneline -30 > /tmp/{project}-gitlog.txt\n\
                 then: {write_hint}"
            ),
            kavach_types::AutoDraftSource::HandlerScan => format!(
                "{header}\nsource: HandlerScan — list HTTP handlers.\n\
                 run: fd -e rs . src/ | xargs rg 'fn handle_|#\\[.*route' | head -20 > /tmp/{project}-handlers.txt\n\
                 then: {write_hint}"
            ),
            kavach_types::AutoDraftSource::TracingScan => format!(
                "{header}\nsource: TracingScan — extract observability calls.\n\
                 run: fd -e rs . src/ | xargs rg 'tracing::(info|warn|error)' | head -20 > /tmp/{project}-tracing.txt\n\
                 then: {write_hint}"
            ),
            kavach_types::AutoDraftSource::RouteScan => format!(
                "{header}\nsource: RouteScan — inspect frontend router for endpoints.\n{scope_hint}\nthen: {write_hint}"
            ),
            kavach_types::AutoDraftSource::TestScan => format!(
                "{header}\nsource: TestScan — find #[test] cases.\n\
                 run: fd -e rs . src/ | xargs rg '#\\[test\\]' -A 2 | head -30 > /tmp/{project}-tests.txt\n\
                 then: {write_hint}"
            ),
            // AutoDraftSource is #[non_exhaustive]: unknown upstream variant
            // routes to spec-author rather than failing the command.
            _ => format!(
                "{header}\nsource: unknown — route to Agent `spec-author`.\n{scope_hint}\nthen: {write_hint}"
            ),
        };
        if let Err(io_err) = print_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        0
    } else {
        let msg = format!("unknown prefix: {prefix}. Valid: spec.prd, arch.trd, arch.data, etc.");
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_draft_human_only() {
        assert_eq!(handle_auto_draft("spec.prd", "test-proj"), 0);
    }

    #[test]
    fn test_auto_draft_invalid() {
        assert_eq!(handle_auto_draft("invalid.prefix", "test-proj"), 1);
    }
}

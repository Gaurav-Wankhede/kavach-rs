//! `[STACK_FIT]` session-start read path — the project's language/tech-stack
//! bound to its non-negotiable boundaries, rendered as Mermaid so the model
//! reasons inside the stack's constraints from the first turn (anti-hallucination
//! when the architecture is complex). VIEW over `stack.*` `app_spec` rows; fail-soft
//! `None` (no stack rows or daemon down ⇒ inject nothing). Agnostic — content is
//! entirely the project's own declared stack.
use kavach_rpc::methods::db::StackRenderResult;

/// Build the `[STACK_FIT]` block for `project_slug`. `None` when the project
/// declares no `stack.*` invariants or the daemon is unreachable.
#[must_use]
pub(super) fn stack_fit_context(project_slug: &str) -> Option<String> {
    if project_slug.is_empty() {
        return None;
    }
    let params = serde_json::json!({ "project_slug": project_slug });
    let res: StackRenderResult = kavach_rpc::client::call("db.stack_render", Some(params)).ok()?;
    let mermaid = res.mermaid?;
    if mermaid.trim().is_empty() {
        return None;
    }
    Some(format!(
        "\n[STACK_FIT] this project's chosen stack bound to its non-negotiable \
         boundaries (each component points at a constraint you must honour — pick \
         APIs/patterns that fit these edges, do not fight the stack):\n\
         ```mermaid\n{}\n```",
        mermaid.trim_end()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_project_returns_none() {
        assert!(stack_fit_context("").is_none());
    }

    #[test]
    fn block_is_none_or_well_formed() {
        // Fail-soft: no daemon/no stack rows ⇒ None; live ⇒ wrapped graph TD.
        match stack_fit_context("kavach-rs") {
            None => {}
            Some(b) => {
                assert!(b.contains("[STACK_FIT]"), "{b}");
                assert!(b.contains("graph TD"), "{b}");
            }
        }
    }
}

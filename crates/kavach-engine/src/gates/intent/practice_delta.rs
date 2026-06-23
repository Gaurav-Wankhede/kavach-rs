//! `[PRACTICE_DELTA]` intent-time injector — this codebase's recurrence-ranked
//! worst-practices contrasted with the research-backed fix that retired each, as a
//! Mermaid `graph LR`. The model sees what THIS repo already learned to stop doing
//! and what replaced it, so it doesn't reintroduce a retired mistake when the
//! architecture gets complex. Read-side VIEW over the anti-pattern ledger;
//! fail-soft `None` (daemon down or empty ledger ⇒ inject nothing).
//! SOURCE: roadmap.unit.mermaid-decision-architecture.

/// Build the `[PRACTICE_DELTA]` block: the recurring anti-patterns RELEVANT to
/// `prompt` (token-overlap on gate/slug/fix) vs their known correct actions.
/// An empty `prompt` (session-start) renders the whole top-N ledger. `None` when
/// the ledger is empty, the daemon is down, or nothing matches the focus.
#[must_use]
pub(in crate::gates) fn practice_delta_block(prompt: &str) -> Option<String> {
    let focus: Vec<String> = if prompt.trim().is_empty() {
        Vec::new()
    } else {
        vec![prompt.to_owned()]
    };
    let params = serde_json::json!({ "focus": focus, "limit": 6 });
    let res: kavach_rpc::methods::db::PracticeRenderResult =
        kavach_rpc::client::call("db.practice_render", Some(params)).ok()?;
    let mermaid = res.mermaid?;
    if mermaid.trim().is_empty() {
        return None;
    }
    Some(format!(
        "\n[PRACTICE_DELTA] mistakes THIS codebase already retired (left) and the \
         research-backed fix that replaced each (right) — do NOT reintroduce a \
         left-side practice; apply the right-side fix by default:\n\
         ```mermaid\n{}\n```",
        mermaid.trim_end()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_is_none_or_well_formed() {
        // Fail-soft contract: with no daemon ⇒ None; with a live daemon holding
        // anti-patterns ⇒ a wrapped `graph LR` block. Never panics, never empty.
        match practice_delta_block() {
            None => {}
            Some(b) => {
                assert!(b.contains("[PRACTICE_DELTA]"), "{b}");
                assert!(b.contains("graph LR"), "{b}");
            }
        }
    }
}

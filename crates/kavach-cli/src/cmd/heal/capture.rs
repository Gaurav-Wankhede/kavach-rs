//! H1 incident capture: gather failure context with NON-AI ops only and write a
//! typed self-heal roadmap card the autonomous loop will dispatch. Kavach never
//! generates the fix — the subscription native agent does, in its own context.
//! SOURCE: decision.heal.self-healing-pipeline-architecture · roadmap heal.unit.incident-capture.

use std::process::Command;

/// Max bytes of a log tail / source excerpt carried into a card — bounds the
/// card size so a runaway log can't blow the context budget (boundary loophole).
const MAX_EXCERPT: usize = 4_000;

/// The fixed contract appended to every self-heal card: tells the claiming agent
/// HOW to heal (root-cause, fix source, 3-witness, never patch the artifact).
const HEAL_CONTRACT: &str = "\n[HEAL_CONTRACT]\n\
    You are the fixer. Root-cause this failure, fix it AT THE SOURCE (never patch the\n\
    built/deployed artifact), then 3-witness verify: rg the change exists, git diff landed,\n\
    cargo check/test exit 0. Loophole-check before any done claim. Open the PR via the heal\n\
    pipeline; do NOT merge — the fail-closed auto-merge gate decides that.\n";

/// Gathered, non-AI failure context for one incident.
pub(super) struct Incident {
    /// Stable incident id (CI run id / finding id) — the card key suffix.
    pub id: String,
    /// One-line failure summary for the card title.
    pub summary: String,
    /// Bounded log tail.
    pub log_tail: String,
    /// Files changed since the diff base (`git diff --name-only`).
    pub changed: Vec<String>,
}

/// Tail the last `MAX_EXCERPT` bytes of `s` on a char boundary (never mid-UTF-8).
fn tail(s: &str) -> String {
    if s.len() <= MAX_EXCERPT {
        return s.to_owned();
    }
    // `char_indices` only yields valid boundaries → no raw byte slicing, no
    // manual boundary walk. Split at the first boundary at/under the byte budget;
    // `split_at` is panic-safe here because the index came from `char_indices`.
    let floor = s.len().saturating_sub(MAX_EXCERPT);
    let kept = s
        .char_indices()
        .find(|&(i, _)| i >= floor)
        .map_or("", |(i, _)| s.split_at(i).1);
    format!("…(truncated)…\n{kept}")
}

/// Changed files vs `base` via `git diff --name-only`. Empty on any git error —
/// the card is still written (a missing diff must not block heal capture).
fn changed_files(base: &str) -> Vec<String> {
    let Ok(out) = Command::new("git")
        .args(["diff", "--name-only", base])
        .output()
    else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::to_owned)
        .collect()
}

/// Build an [`Incident`] from the raw inputs (log text already read by the caller).
pub(super) fn gather(id: &str, summary: &str, log: &str, diff_base: &str) -> Incident {
    Incident {
        id: id.to_owned(),
        summary: summary.to_owned(),
        log_tail: tail(log),
        changed: changed_files(diff_base),
    }
}

/// Render the card content: gathered context + the fixed heal contract.
pub(super) fn card_content(inc: &Incident) -> String {
    let changed = if inc.changed.is_empty() {
        "(none detected)".to_owned()
    } else {
        inc.changed.join("\n  ")
    };
    format!(
        "[INCIDENT]\nid: {}\nsummary: {}\n\n[CHANGED_FILES]\n  {}\n\n[LOG_TAIL]\n{}\n{}",
        inc.id, inc.summary, changed, inc.log_tail, HEAL_CONTRACT
    )
}

/// The card key for an incident — deterministic, so re-capture UPDATES the same
/// card (idempotent on incident id; replay loophole closed).
pub(super) fn card_key(id: &str) -> String {
    format!("heal.incident.{id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_is_deterministic_for_idempotent_recapture() {
        assert_eq!(card_key("run-42"), card_key("run-42"));
        assert_eq!(card_key("run-42"), "heal.incident.run-42");
    }

    #[test]
    fn content_carries_log_tail_and_changed_and_contract() {
        let inc = gather("run-7", "smoke test failed", "boom\nstack\ntrace", "HEAD~1");
        let c = card_content(&inc);
        assert!(c.contains("smoke test failed"), "{c}");
        assert!(c.contains("trace"), "log tail present: {c}");
        assert!(c.contains("[HEAL_CONTRACT]"), "contract present: {c}");
    }

    #[test]
    fn oversized_log_is_truncated_at_a_char_boundary() {
        // Multi-byte chars at the cut point must not panic (boundary loophole).
        let big = "é".repeat(MAX_EXCERPT); // 2 bytes each → well over the budget
        let t = tail(&big);
        assert!(t.len() <= MAX_EXCERPT + 32, "bounded: {} bytes", t.len());
        assert!(t.starts_with("…(truncated)…"), "marks truncation");
    }
}

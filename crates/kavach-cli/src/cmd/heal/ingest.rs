//! H5 ingestion bridge (host-side): the link between the CI self-heal issue
//! queue (H2, runner) and the local roadmap card (H1, host). Polls OPEN
//! `self-heal`-labelled GitHub issues, captures each as a local card via the RPC
//! single-writer path, then relabels the issue `self-heal-queued` so it is
//! ingested EXACTLY ONCE. Kavach runs only `gh` here — never an LLM.
//! SOURCE: roadmap heal.unit.e2e-wire-verify · .github/workflows/self-heal.yml.

mod parse;

use super::capture_incident;
use crate::cmd::io_safe::{IoExit, into_exit_code, print_or_exit};
use parse::parse_incident;
use std::process::Command;

/// Label the CI workflow attaches to new incidents.
const OPEN_LABEL: &str = "self-heal";
/// Label applied after a successful local capture → idempotency marker so the
/// next poll skips it (queried as `--label self-heal --label -self-heal-queued`
/// is not a gh filter, so we filter client-side on the already-queued label too).
const QUEUED_LABEL: &str = "self-heal-queued";

/// `kavach heal ingest` entry. Exit 0 on a clean poll (even with 0 issues).
pub(crate) fn run(project: &str) -> i32 {
    match run_inner(project) {
        Ok(()) => 0,
        Err(io) => into_exit_code(io),
    }
}

fn run_inner(project: &str) -> Result<(), IoExit> {
    let issues = open_issues();
    let mut ingested = 0_u32;
    for (number, body) in &issues {
        let Some(inc) = parse_incident(body) else {
            print_or_exit(&format!("[heal ingest] skip #{number}: no [INCIDENT] block"))?;
            continue;
        };
        // Capture locally via the H1 RPC path (idempotent on inc.id).
        let code = capture_incident(project, &inc.id, &inc.summary, body, "HEAD~1");
        if code != 0 {
            print_or_exit(&format!("[heal ingest] WARN #{number}: capture returned {code}"))?;
            continue;
        }
        // Relabel so this issue is never re-ingested (exactly-once). A relabel
        // failure leaves the OPEN_LABEL on, so the next run retries — capture is
        // idempotent, so a retry can't double-write. We surface but do not abort.
        if !relabel_queued(*number) {
            print_or_exit(&format!(
                "[heal ingest] WARN #{number}: captured but relabel failed; will retry (capture is idempotent)"
            ))?;
        }
        ingested = ingested.saturating_add(1);
    }
    print_or_exit(&format!("[heal ingest] done: {ingested} incident(s) ingested"))
}

/// Open issues still labelled `self-heal` but NOT yet `self-heal-queued`.
/// Empty on any `gh`/parse error (a poll failure ingests nothing — never a
/// half-state). Returns `(issue_number, body)` pairs.
fn open_issues() -> Vec<(u64, String)> {
    let Ok(out) = Command::new("gh")
        .args([
            "issue", "list", "--state", "open", "--label", OPEN_LABEL, "--json", "number,body,labels",
        ])
        .output()
    else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    let Ok(rows) = serde_json::from_slice::<Vec<IssueRow>>(&out.stdout) else {
        return Vec::new();
    };
    rows.into_iter()
        .filter(|r| !r.labels.iter().any(|l| l.name == QUEUED_LABEL))
        .map(|r| (r.number, r.body))
        .collect()
}

/// Add the `self-heal-queued` label. True on success.
fn relabel_queued(number: u64) -> bool {
    Command::new("gh")
        .args(["issue", "edit", &number.to_string(), "--add-label", QUEUED_LABEL])
        .output()
        .is_ok_and(|o| o.status.success())
}

#[derive(serde::Deserialize)]
struct IssueRow {
    number: u64,
    body: String,
    labels: Vec<IssueLabel>,
}

#[derive(serde::Deserialize)]
struct IssueLabel {
    name: String,
}

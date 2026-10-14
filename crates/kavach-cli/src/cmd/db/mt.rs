//! `kavach db mt` — author and validate `MicroTicket v1` rows.
//!
//! The plain `kavach db write` accepts free text, which cannot enforce the
//! ticket contract. This verb is the fail-closed boundary: it takes typed
//! fields, validates against `kavach_db::microticket`, and only then writes.
//!
//! WHY a separate verb rather than validating inside `db write`: `db write` is
//! the general memory-write path (decisions, research, patterns) and a schema
//! would reject legitimate prose rows. A ticket is a different KIND of thing and
//! earns its own contract.
//!
//! WHAT IS ENFORCED: caps, single-sentence goals, non-empty verify, mandatory
//! scope AND do_not_touch, run-required-for-cmd/test, manual_by-required-for-
//! manual, and the depends_on/conflicts_with contradiction check.
//!
//! WHAT IS NOT: quality. A ticket that validates can still be semantically
//! wrong — see the Constraint Tax note in `kavach_db::microticket`. Never treat
//! a successful `mt new` as evidence the ticket is correct.

use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use kavach_db::microticket::{
    Change, MicroTicket, Modality, Op, Source, SourceKind, TicketKind, VerifyMethod, VerifyStep,
};
use kavach_db::db::entries::EntryStore;

/// Flags as parsed by clap, kept as strings and narrowed HERE so the enum
/// narrowing errors are reported with the same shape as every other validation
/// failure (an author fixes all of them at once).
pub(crate) struct MtArgs {
    pub project: String,
    pub key: String,
    pub kind: String,
    pub title: String,
    pub goal: String,
    pub modality: String,
    pub op: String,
    pub when: String,
    pub then: String,
    pub scope: Vec<String>,
    pub do_not_touch: Vec<String>,
    pub verify_cmd: Vec<String>,
    pub verify_test: Vec<String>,
    pub verify_assert: Vec<String>,
    pub verify_manual: Vec<String>,
    pub source_kind: Option<String>,
    pub source_ref: Option<String>,
    pub source_note: Option<String>,
    pub depends_on: Vec<String>,
    pub conflicts_with: Vec<String>,
    pub supersedes: Vec<String>,
    pub manual_by: Option<String>,
    pub priority: Option<i64>,
    pub exec_prompt: Option<String>,
    pub dry_run: bool,
}

fn narrow<T: Copy + std::fmt::Debug>(
    field: &str,
    raw: &str,
    table: &[(&str, T)],
    errs: &mut Vec<String>,
) -> Option<T> {
    match table.iter().find(|(k, _)| *k == raw) {
        Some((_, v)) => Some(*v),
        None => {
            let allowed: Vec<&str> = table.iter().map(|(k, _)| *k).collect();
            errs.push(format!(
                "{field}: '{raw}' is not a valid value — expected one of: {}",
                allowed.join(" | ")
            ));
            None
        }
    }
}

/// Build the ticket from flags. Every problem is collected, never raised early:
/// an author fixing one field per round-trip is the friction that pushes people
/// back to free text.
fn build(args: &MtArgs) -> Result<MicroTicket, Vec<String>> {
    let mut errs: Vec<String> = Vec::new();

    let kind = narrow(
        "kind",
        &args.kind,
        &[
            ("bug", TicketKind::Bug),
            ("feature", TicketKind::Feature),
            ("chore", TicketKind::Chore),
            ("refactor", TicketKind::Refactor),
            ("perf", TicketKind::Perf),
            ("docs", TicketKind::Docs),
            ("test", TicketKind::Test),
        ],
        &mut errs,
    );
    let modality = narrow(
        "modality",
        &args.modality,
        &[("MUST", Modality::Must), ("SHOULD", Modality::Should), ("MAY", Modality::May)],
        &mut errs,
    );
    let op = narrow(
        "change.op",
        &args.op,
        &[
            ("add", Op::Add),
            ("edit", Op::Edit),
            ("delete", Op::Delete),
            ("move", Op::Move),
        ],
        &mut errs,
    );

    let mut verify: Vec<VerifyStep> = Vec::new();
    // `--verify-cmd "cmd::expect"` and `--verify-test "path::expect"` carry the
    // command and its expectation as one value so the pairing cannot be lost.
    for (prefix, method, items) in [
        ("verify-cmd", VerifyMethod::Cmd, &args.verify_cmd),
        ("verify-test", VerifyMethod::Test, &args.verify_test),
        ("verify-assert", VerifyMethod::Assert, &args.verify_assert),
    ] {
        for raw in items {
            match raw.split_once("::") {
                Some((run, expect)) => verify.push(VerifyStep {
                    method,
                    run: Some(run.trim().to_owned()),
                    expect: expect.trim().to_owned(),
                }),
                None => errs.push(format!(
                    "{prefix}: expected 'RUN::EXPECT' but got '{raw}' with no '::' separator"
                )),
            }
        }
    }
    for expect in &args.verify_manual {
        verify.push(VerifyStep {
            method: VerifyMethod::Manual,
            run: None,
            expect: expect.trim().to_owned(),
        });
    }

    // Source is a pair or nothing: a kind without a ref is a dangling claim.
    let source = match (args.source_kind.as_deref(), args.source_ref.as_deref()) {
        (None, None) => None,
        (Some(k), Some(r)) => narrow(
            "source.kind",
            k,
            &[
                ("crate", SourceKind::Crate),
                ("file", SourceKind::File),
                ("url", SourceKind::Url),
                ("doc", SourceKind::Doc),
                ("test", SourceKind::Test),
                ("db", SourceKind::Db),
            ],
            &mut errs,
        )
        .map(|kind| Source {
            kind,
            r#ref: r.to_owned(),
            note: args.source_note.clone(),
        }),
        (Some(_), None) => {
            errs.push("source: --source-kind given without --source-ref".to_owned());
            None
        }
        (None, Some(_)) => {
            errs.push("source: --source-ref given without --source-kind".to_owned());
            None
        }
    };

    let Some(kind) = kind else {
        return Err(errs);
    };
    let Some(modality) = modality else {
        return Err(errs);
    };
    let Some(op) = op else {
        return Err(errs);
    };

    let ticket = MicroTicket {
        kind,
        title: args.title.clone(),
        goal: args.goal.clone(),
        modality,
        change: Change {
            op,
            when: args.when.clone(),
            then: args.then.clone(),
        },
        scope: args.scope.clone(),
        do_not_touch: args.do_not_touch.clone(),
        verify,
        source,
        depends_on: args.depends_on.clone(),
        conflicts_with: args.conflicts_with.clone(),
        supersedes: args.supersedes.clone(),
        manual_by: args.manual_by.clone(),
    };

    // Fold the schema's own violations into the same error list.
    if let Err(schema_errs) = ticket.validate() {
        errs.extend(schema_errs.iter().map(ToString::to_string));
    }
    if errs.is_empty() {
        Ok(ticket)
    } else {
        Err(errs)
    }
}

/// Render the stored body. JSON is the canonical form; the leading
/// `MTICKET v1` marker lets a reader (human or agent) detect the shape without
/// guessing, and keeps `kavach db query` output self-describing.
fn render_body(ticket: &MicroTicket) -> String {
    match ticket.to_json() {
        Ok(j) => format!("MTICKET v1\n{j}"),
        // Serializing a ticket that built successfully cannot fail; but a body
        // that lost its ticket would be a silent downgrade, so fail loudly.
        Err(e) => format!("MTICKET v1\n{{\"serialization_error\":\"{e}\"}}"),
    }
}

/// Read the project's roadmap rows for `mt check`. Read-only direct open — WAL
/// allows concurrent readers, so this never races the daemon's write lock.
fn fetch_roadmap(project: &str) -> Result<Vec<kavach_db::MemoryEntry>, String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    rt.block_on(async {
        let store = super::rpc_client::open_turso_resilient().await?;
        store
            .list_by_category(project, "roadmap")
            .await
            .map_err(|e| e.to_string())
    })
}

pub(crate) fn run(args: &MtArgs) -> i32 {
    let ticket = match build(args) {
        Ok(t) => t,
        Err(errs) => {
            let msg = format!(
                "error: micro-ticket rejected ({} problem(s)):\n  - {}\n\n\
                 See `kavach db mt new --help` for the field contract.",
                errs.len(),
                errs.join("\n  - ")
            );
            return match ewrite_or_exit(&msg) {
                Ok(()) => 1,
                Err(io_err) => into_exit_code(io_err),
            };
        }
    };

    let body = render_body(&ticket);
    if args.dry_run {
        return match print_or_exit(&format!(
            "dry-run OK — {}\n{body}",
            ticket.title
        )) {
            Ok(()) => 0,
            Err(io_err) => into_exit_code(io_err),
        };
    }

    super::write::run(&super::rpc_client::WriteRequest {
        project: &args.project,
        category: "roadmap",
        key: &args.key,
        title: &ticket.title,
        content: Some(&body),
        new: true,
        update_key: None,
        priority: args.priority,
        exec_prompt: args.exec_prompt.as_deref(),
        depends_on: &args.depends_on,
    })
}

/// `kavach db mt check` — revalidate stored tickets, so a row written by an
/// older/looser path is surfaced rather than trusted.
pub(crate) fn check(project: &str, key_filter: Option<&str>) -> i32 {
    let rows = match fetch_roadmap(project) {
        Ok(r) => r,
        Err(e) => {
            return match ewrite_or_exit(&format!("error: {e}")) {
                Ok(()) => 1,
                Err(io_err) => into_exit_code(io_err),
            };
        }
    };
    let mut checked = 0usize;
    let mut bad = 0usize;
    for row in &rows {
        if key_filter.is_some_and(|f| !row.entry_key.contains(f)) {
            continue;
        }
        if !row.content.trim_start().starts_with("MTICKET v1") {
            continue;
        }
        checked = checked.saturating_add(1);
        let json = row.content.trim_start().strip_prefix("MTICKET v1").unwrap_or_default();
        match MicroTicket::from_json(json) {
            Err(e) => {
                bad = bad.saturating_add(1);
                eprintln!("[invalid] {}: unparseable ticket JSON — {e}", row.entry_key);
            }
            Ok(t) => {
                if let Err(errs) = t.validate() {
                    bad = bad.saturating_add(1);
                    eprintln!("[invalid] {}", row.entry_key);
                    for e in &errs {
                        eprintln!("    {e}");
                    }
                }
            }
        }
    }
    let msg = format!("checked {checked} micro-ticket(s), {bad} invalid");
    match print_or_exit(&msg) {
        Ok(()) => i32::from(bad > 0),
        Err(io_err) => into_exit_code(io_err),
    }
}
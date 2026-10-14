// Unified context payload for AI agent harness — single JSON with kanban, session, phase, loop state.
// SOURCE: https://dev.to/uenyioha/writing-cli-tools-that-ai-agents-actually-want-to-use-39no
// PATTERN: Single structured JSON payload per GitLab glab CLI pattern.
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(super) fn run(
    project: &str,
    limit: usize,
    status_filter: Option<&str>,
    key_filter: Option<&str>,
    delta: bool,
    full: bool,
) -> i32 {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            let msg = format!(r#"{{"error":"tokio: {e}"}}"#);
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    runtime.block_on(async { run_async(project, limit, status_filter, key_filter, delta, full).await })
}

async fn run_async(
    project_slug: &str,
    limit: usize,
    status_filter: Option<&str>,
    key_filter: Option<&str>,
    delta: bool,
    full: bool,
) -> i32 {
    // Session state
    let session = kavach_session::get_or_create_session();

    // DB connection. Resilient open: this runs inside a hook child, so a daemon
    // mid-restart can transiently hold the RocksDB lock — wait it out rather than
    // race-and-fail (rca-stop-hook-database-lock-direct-open-bypasses-rpc).
    let db = match kavach_db::open_default_resilient().await {
        Ok(d) => d,
        Err(e) => {
            let msg = format!(r#"{{"error":"db: {e}"}}"#);
            if let Err(io_err) = print_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };

    // Project lookup
    let project = match kavach_db::project_get_by_slug(&db, project_slug).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            let msg = format!(r#"{{"error":"project not found: {project_slug}"}}"#);
            if let Err(io_err) = print_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
        Err(e) => {
            let msg = format!(r#"{{"error":"{e}"}}"#);
            if let Err(io_err) = print_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };

    let Some(project_id) = project.id else {
        if let Err(io_err) = print_or_exit(r#"{"error":"project has no id"}"#) {
            return into_exit_code(io_err);
        }
        return 1;
    };

    // Kanban items
    let roadmap = match kavach_db::list_by_project(&db, "roadmap", &project_id).await {
        Ok(rows) => rows,
        Err(e) => {
            let msg = format!(r#"{{"error":"{e}"}}"#);
            if let Err(io_err) = print_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };

    // 3-state workflow band: ready → in-flight → completed-pending-verify.
    // Statuses as tokens: the `--status` keyword is lexed once into a
    // predicate, and every row matches on its typed lane (discriminant
    // compare) — the `== "todo"` string scans are gone. `Never` (unknown
    // filter word) matches zero rows, preserving exact-match semantics.
    let status_pred = kavach_db::StatusPred::from_opt(status_filter);
    let mut open: Vec<_> = roadmap
        .iter()
        .filter(|e| {
            e.effective_status_typed()
                .is_some_and(|s| s != kavach_types::MemoryStatus::Verified)
        })
        .filter(|e| status_pred.matches_typed(e.effective_status_typed()))
        .filter(|e| key_filter.is_none_or(|k| e.entry_key.contains(k)))
        .collect();

    // Sort: in_progress first
    open.sort_by_key(|e| {
        i32::from(e.effective_status_typed() != Some(kavach_types::MemoryStatus::InProgress))
    });

    let total = open.len();
    let displayed: Vec<_> = if limit == 0 {
        open.into_iter().cloned().collect()
    } else {
        open.into_iter().take(limit).cloned().collect()
    };

    let opts = PayloadOpts {
        limit,
        total,
        delta,
        full,
    };
    build_and_print_payload(project_slug, &session, &roadmap, &displayed, opts)
}

#[derive(Clone, Copy)]
struct PayloadOpts {
    limit: usize,
    total: usize,
    delta: bool,
    full: bool,
}

fn snapshot_file(project: &str) -> std::path::PathBuf {
    let home = std::env::var("HOME").map_or_else(
        |_| {
            std::env::var("USERPROFILE")
                .map_or_else(|_| std::path::PathBuf::from("."), std::path::PathBuf::from)
        },
        std::path::PathBuf::from,
    );
    home.join(".kavach").join(format!("last_context_{project}.json"))
}

#[expect(
    clippy::too_many_lines,
    reason = "Constructs comprehensive context payload with optional JSON delta computation"
)]
fn build_and_print_payload(
    project_slug: &str,
    session: &kavach_session::SessionState,
    roadmap: &[kavach_db::MemoryEntry],
    displayed: &[kavach_db::MemoryEntry],
    opts: PayloadOpts,
) -> i32 {
    // Build kanban JSON items
    let items_json: Vec<String> = displayed
        .iter()
        .map(|e| {
            format!(
                r#"{{"key":"{}","status":"{}","title":"{}"}}"#,
                e.entry_key.replace('"', r#"\""#),
                e.effective_status(),
                e.title.replace('"', r#"\""#)
            )
        })
        .collect();

    let mut todo_count = 0usize;
    let mut in_progress_count = 0usize;
    let mut done_count = 0usize;
    let mut verified_count = 0usize;
    let mut unparseable_count = 0usize;
    for e in roadmap {
        // Single typed tally: discriminant match, no string scans.
        match e.effective_status_typed() {
            Some(kavach_types::MemoryStatus::Todo) => {
                todo_count = todo_count.saturating_add(1);
            }
            Some(kavach_types::MemoryStatus::InProgress) => {
                in_progress_count = in_progress_count.saturating_add(1);
            }
            Some(kavach_types::MemoryStatus::Done) => {
                done_count = done_count.saturating_add(1);
            }
            Some(kavach_types::MemoryStatus::Verified) => {
                verified_count = verified_count.saturating_add(1);
            }
            // Unknown literals match no lane (same as the old string scans).
            Some(_) | None => unparseable_count = unparseable_count.saturating_add(1),
        }
    }
    // Self-consistency: every row lands in exactly one lane. A nonzero
    // unparseable remainder means a new status literal reached the board.
    debug_assert_eq!(
        todo_count
            .saturating_add(in_progress_count)
            .saturating_add(done_count)
            .saturating_add(verified_count)
            .saturating_add(unparseable_count),
        roadmap.len(),
        "context kanban counts must partition the roadmap",
    );

    let payload = format!(
        r#"{{"project":"{project_slug}","session":{{"id":"{sid}","turn":{turn},"phase":"{phase}"}},"loop":{{"active":{active},"target":"{target}","iteration":{iter},"max":{max}}},"kanban":{{"in_progress":{ip},"todo":{td},"done":{dn},"verified":{vf}}},"items":[{items}],"total":{total},"displayed":{disp},"has_more":{more}}}"#,
        sid = session.session_id,
        turn = session.turn_count,
        phase = session.current_phase,
        active = session.loop_active,
        target = session.loop_target,
        iter = session.loop_iteration,
        max = session.loop_max_iterations,
        ip = in_progress_count,
        td = todo_count,
        dn = done_count,
        vf = verified_count,
        items = items_json.join(","),
        total = opts.total,
        disp = displayed.len(),
        more = opts.limit > 0 && opts.total > opts.limit,
    );

    let output = if opts.full {
        if opts.delta {
            let snap_path = snapshot_file(project_slug);
            let prev = std::fs::read_to_string(&snap_path).unwrap_or_default();
            if let Some(parent) = snap_path.parent()
                && let Err(e) = std::fs::create_dir_all(parent)
            {
                eprintln!("warning: failed to create context snap dir: {e}");
            }
            if let Err(e) = std::fs::write(&snap_path, &payload) {
                eprintln!("warning: failed to write context snapshot: {e}");
            }

            if prev.trim() == payload.trim() {
                format!(r#"{{"project":"{project_slug}","delta":true,"changed":{{}}}}"#)
            } else if let (Ok(prev_val), Ok(curr_val)) = (
                serde_json::from_str::<serde_json::Value>(&prev),
                serde_json::from_str::<serde_json::Value>(&payload),
            ) {
                let mut diff = serde_json::Map::new();
                if let (Some(prev_obj), Some(curr_obj)) = (prev_val.as_object(), curr_val.as_object()) {
                    for (k, v) in curr_obj {
                        if prev_obj.get(k) != Some(v) {
                            diff.insert(k.clone(), v.clone());
                        }
                    }
                }
                serde_json::json!({
                    "project": project_slug,
                    "delta": true,
                    "changed": diff
                }).to_string()
            } else {
                payload
            }
        } else {
            payload
        }
    } else {
        let active_target = if session.loop_target.is_empty() {
            displayed.first().map_or("none", |c| c.entry_key.as_str())
        } else {
            &session.loop_target
        };
        format!(
            "[STATE] project={project_slug} phase={} target={active_target} kanban={{ip:{in_progress_count},td:{todo_count},dn:{done_count},vf:{verified_count}}} loop={{active:{},iter:{}/{}}}",
            session.current_phase,
            session.loop_active,
            session.loop_iteration,
            session.loop_max_iterations
        )
    };

    if let Err(io_err) = print_or_exit(&output) {
        return into_exit_code(io_err);
    }
    0
}

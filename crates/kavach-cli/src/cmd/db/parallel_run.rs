//! `kavach db parallel-run` — run multiple runnable kanban cards in parallel,
//! respecting dependency DAG and max concurrency.
use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};
use kavach_db::dual_write::MemoryEntry;
use kavach_db::turso::entries::EntryStore;
use kavach_db::turso::TursoStore;
use std::collections::{HashMap, VecDeque};
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Outcome of picking servable cards from the roadmap.
enum Pick<'a> {
    /// Cards that can be run now (all dependencies satisfied).
    Ready(Vec<&'a MemoryEntry>),
    /// No runnable cards at the moment.
    Empty,
}

/// Pick all `todo` roadmap cards whose dependencies are satisfied.
fn pick_ready<'a>(rows: &'a [MemoryEntry], _store: &TursoStore) -> Pick<'a> {
    let mut ready = Vec::new();
    let mut key_to_row = HashMap::new();

    for row in rows {
        key_to_row.insert(&row.entry_key, row);
    }

    for row in rows {
        if row.effective_status_typed() != Some(kavach_types::MemoryStatus::Todo) {
            continue;
        }

        // Check if all dependencies are done/verified
        let content = &row.content;
        let mut all_deps_done = true;

        // Parse DEPENDS_ON lines from content
        for line in content.lines() {
            if let Some(deps_str) = line.strip_prefix("DEPENDS_ON:") {
                for dep in deps_str.split(',') {
                    let dep = dep.trim();
                    if dep.is_empty() {
                        continue;
                    }
                    // dep could be bare key or qualified
                    let dep_key = if dep.contains('/') {
                        dep.to_string()
                    } else {
                        format!(
                            "{}/{}/{}",
                            row.project.as_str(),
                            row.category.as_deref().unwrap_or(""),
                            dep
                        )
                    };

                    if let Some(dep_row) = key_to_row.get(&dep_key) {
                        if dep_row.effective_status_typed() != Some(kavach_types::MemoryStatus::Done)
                            && dep_row.effective_status_typed() != Some(kavach_types::MemoryStatus::Verified)
                        {
                            all_deps_done = false;
                            break;
                        }
                    } else {
                        // Dependency not in current query - check DB
                        // For now, assume not done if not in our rows
                        all_deps_done = false;
                        break;
                    }
                }
            }
        }

        if all_deps_done {
            ready.push(row);
        }
    }

    if ready.is_empty() {
        Pick::Empty
    } else {
        Pick::Ready(ready)
    }
}

/// Run a single card: mark in_progress, execute verification, mark done/verified.
async fn run_card(store: &TursoStore, card: &MemoryEntry, verify_cmd: Option<&str>) -> Result<(), String> {
    // Mark in_progress
    store
        .upsert_entry(&MemoryEntry {
            entry_status: Some(kavach_types::MemoryStatus::InProgress.to_string()),
            ..card.clone()
        })
        .await
        .map_err(|e| format!("Failed to mark in_progress: {e}"))?;

    // If no verify_cmd, just mark done
    let verify_cmd = verify_cmd.or(card.exec_prompt.as_deref());

    if let Some(cmd) = verify_cmd {
        // Execute verification command
        let output = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .map_err(|e| format!("Failed to execute verify command: {e}"))?;

        if !output.status.success() {
            // Mark back to todo on failure
            let mut updated = card.clone();
            updated.entry_status = Some(kavach_types::MemoryStatus::Todo.to_string());
            store.upsert_entry(&updated).await
                .map_err(|e| format!("Failed to revert status: {e}"))?;

            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Verification failed: {stderr}"));
        }
    }

    // Mark done (verified if verification passed)
    let final_status = if verify_cmd.is_some() {
        kavach_types::MemoryStatus::Verified
    } else {
        kavach_types::MemoryStatus::Done
    };

    let mut updated = card.clone();
    updated.entry_status = Some(final_status.to_string());
    store.upsert_entry(&updated).await
        .map_err(|e| format!("Failed to mark done: {e}"))?;

    Ok(())
}

/// Run parallel execution of ready cards.
async fn run_parallel(
    store: &TursoStore,
    project: &str,
    max_parallel: usize,
    verify_cmd: Option<String>,
) -> Result<usize, String> {
    let semaphore = Arc::new(Semaphore::new(max_parallel));
    let mut total_completed = 0;

    loop {
        // Fetch fresh rows from DB each iteration
        let rows = store
            .list_by_category(project, "roadmap")
            .await
            .map_err(|e| format!("Failed to list roadmap: {e}"))?;

        // Pick ready cards
        let pick = pick_ready(&rows, store);
        let ready = match pick {
            Pick::Ready(r) => r,
            Pick::Empty => break,
        };

        if ready.is_empty() {
            break;
        }

        // Process ready cards in parallel
        // Extract owned data from ready cards to avoid borrowing issues with tokio::spawn
        let cards: Vec<MemoryEntry> = ready.into_iter().cloned().collect();

        // Process ready cards in parallel
        let mut handles = VecDeque::new();
        let semaphore = semaphore.clone();
        let store = store.clone();
        let verify_cmd = verify_cmd.clone();

        for card in cards {
            let permit = semaphore.clone().acquire_owned().await.map_err(|e| e.to_string())?;
            let store = store.clone();
            let verify_cmd = verify_cmd.clone();
            let key = card.entry_key.clone();

            let handle = tokio::spawn(async move {
                let _permit = permit; // Hold permit for duration
                let result = run_card(&store, &card, verify_cmd.as_deref()).await;
                (key, result)
            });

            handles.push_back(handle);
        }

        // Wait for all handles
        while let Some(handle) = handles.pop_front() {
            match handle.await {
                Ok((key, Ok(()))) => {
                    println!("✓ Completed: {}", key);
                    total_completed += 1;
                }
                Ok((key, Err(e))) => {
                    eprintln!("✗ Failed: {} — {}", key, e);
                }
                Err(e) => {
                    eprintln!("✗ Task panicked: {}", e);
                }
            }
        }
    }

    Ok(total_completed)
}

/// `kavach db parallel-run --project P --max-parallel N`:
/// Run all runnable roadmap cards in parallel, respecting dependencies.
pub(super) fn run(project: &str, max_parallel: usize, verify_cmd: Option<String>) -> i32 {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => return render_err(&format!("error: tokio runtime: {e}")),
    };

    runtime.block_on(async {
        let store = match super::rpc_client::open_turso_resilient().await {
            Ok(s) => s,
            Err(e) => return render_err(&format!("error: open kavach.db: {e}")),
        };

        match run_parallel(&store, project, max_parallel, verify_cmd).await {
            Ok(completed) => {
                print_or_exit(&format!("Parallel run completed: {} cards processed", completed)).unwrap();
                0
            }
            Err(e) => render_err(&format!("error: parallel run failed: {e}")),
        }
    })
}

fn render_err(msg: &str) -> i32 {
    match ewrite_or_exit(msg) {
        Ok(()) => 1,
        Err(io_err) => into_exit_code(io_err),
    }
}
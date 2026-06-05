// MOLECULE: run modal — confirms a Claude Code subprocess launch.
// Shows pre-flight, lets user edit prompt + model override, kicks off the run.
//
// ALGO: ConcurrencyCappedDispatch
// PROBLEM_CLASS: stream
// REJECTED: [{"name":"unbounded_spawn","reason":"hits Max plan rate limits"},{"name":"single_serial","reason":"too slow for the user goal"}]
// TIME: O(1) dispatch | SPACE: O(runs)
// YEAR: 2026 | SEARCHED: 2026-05
// TRADEOFF: 2-cap is conservative; user can override
// BENCHMARK: https://amux.io/guides/claude-code-headless/
// SOURCE: https://code.claude.com/docs/en/headless
use chrono::Utc;
use dioxus::prelude::*;
use std::path::PathBuf;

use crate::runner::policy::pre_flight;
use crate::runner::spawn::{SpawnRequest, spawn as spawn_runner};
use crate::state::{RUN_TARGET, RUNS, RunHandle, RunStatus};

const MAX_CONCURRENT: usize = 2;

fn active_run_count(runs: &std::collections::HashMap<String, RunHandle>) -> usize {
    runs.values()
        .filter(|r| matches!(r.status, RunStatus::Running))
        .count()
}

fn home_runs_dir() -> PathBuf {
    dirs::home_dir().map_or_else(
        || PathBuf::from("/tmp/kavach-runs"),
        |h| h.join(".kavach").join("runs"),
    )
}

#[component]
pub fn RunModal() -> Element {
    let snap = RUN_TARGET.read().clone();
    let Some(target) = snap else { return rsx! {} };
    let pf = use_memo(pre_flight);
    let mut prompt = use_signal(|| {
        format!(
            "# {}\n\n{}\n\nWork on this task.",
            target.title, target.content
        )
    });
    let mut model_override = use_signal(String::new);
    let mut error = use_signal(String::new);
    let entry_for_run = target.clone();
    let entry_key_for_close = target.key;

    rsx! {
        div { class: "modal-backdrop", onclick: move |_| { *RUN_TARGET.write() = None; },
            div { class: "modal modal-wide", onclick: move |e| e.stop_propagation(),
                h2 { "Run task in Claude Code" }
                div { class: "preflight",
                    {
                        let pf_val = pf();
                        match pf_val.claude_path.as_ref() {
                            Some(path) => rsx! {
                                div { class: "preflight-ok",
                                    "✓ claude found: "
                                    code { "{path}" }
                                    if let Some(v) = pf_val.version.as_ref() {
                                        " ({v})"
                                    }
                                }
                            },
                            None => rsx! {
                                div { class: "preflight-err",
                                    "✗ claude binary not on PATH. Install via "
                                    code { "npm i -g @anthropic-ai/claude-code" }
                                    " and run "
                                    code { "claude login" }
                                    "."
                                }
                            },
                        }
                    }
                    if pf().api_key_set {
                        div { class: "preflight-warn",
                            "⚠ ANTHROPIC_API_KEY is set in your environment — runs will bill against the API key, not your Max subscription. Unset to use Max."
                        }
                    } else {
                        div { class: "preflight-ok",
                            "✓ ANTHROPIC_API_KEY not set — runs will use your Claude Max subscription."
                        }
                    }
                }
                label { "Prompt"
                    textarea {
                        class: "textarea",
                        rows: "10",
                        value: "{prompt}",
                        oninput: move |e| prompt.set(e.value()),
                    }
                }
                label { "Model override (leave empty to use your settings.json default)"
                    input {
                        class: "input",
                        placeholder: "e.g. claude-opus-4-7  or leave empty",
                        value: "{model_override}",
                        oninput: move |e| model_override.set(e.value()),
                    }
                }
                if !error.read().is_empty() {
                    div { class: "modal-error", "{error}" }
                }
                div { class: "modal-actions",
                    button {
                        class: "btn",
                        onclick: move |_| { *RUN_TARGET.write() = None; },
                        "Cancel"
                    }
                    button {
                        class: "btn btn-success",
                        disabled: pf().claude_path.is_none(),
                        onclick: move |_| {
                            let runs_snapshot = RUNS.read().clone();
                            if active_run_count(&runs_snapshot) >= MAX_CONCURRENT {
                                error.set(format!("max {MAX_CONCURRENT} concurrent runs; wait or cancel one"));
                                return;
                            }
                            let entry = entry_for_run.clone();
                            let prompt_val = prompt.read().clone();
                            let model_val = {
                                let s = model_override.read().clone();
                                if s.trim().is_empty() { None } else { Some(s) }
                            };
                            let ts = Utc::now().format("%Y%m%d-%H%M%S").to_string();
                            let branch = format!("run/{}-{}", entry.key, ts);
                            let worktree_path = home_runs_dir().join(format!("{}-{}", entry.key, ts));
                            // Detect project workdir: ask SurrealDB via blocking minimal query —
                            // but we don't have the workdir handy in EntryRef. Use the kavach-rs
                            // workspace root as a sane default for the kavach-rs project. For
                            // other projects the user can configure this later.
                            let workdir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
                            let req = SpawnRequest {
                                project_workdir: workdir,
                                branch: branch.clone(),
                                worktree_path: worktree_path.clone(),
                                prompt: prompt_val,
                                model_override: model_val,
                            };
                            let entry_key = entry.key.clone();
                            let project_slug = entry.project_slug;
                            let worktree_str = worktree_path.to_string_lossy().to_string();
                            let initial_handle = RunHandle {
                                entry_key: entry_key.clone(),
                                project_slug,
                                branch,
                                worktree_path: worktree_str,
                                // Queued until the child actually spawns
                                // (flips to Running in the Ok arm below).
                                status: RunStatus::Queued,
                                started_at: Some(Utc::now()),
                                finished_at: None,
                                events: Vec::new(),
                                cost_usd: None,
                                child_pid: None,
                            };
                            RUNS.write().insert(entry_key.clone(), initial_handle);
                            *RUN_TARGET.write() = None;
                            spawn(async move {
                                match spawn_runner(&req) {
                                    Ok(running) => {
                                        // Child is live: Queued -> Running, retain pid
                                        // so cancel_run() can terminate it.
                                        if let Some(h) = RUNS.write().get_mut(&entry_key) {
                                            h.status = RunStatus::Running;
                                            h.child_pid = Some(running.child_pid);
                                        }
                                        // Drain events into the run handle. We do this in a
                                        // dedicated thread (mpsc::Receiver isn't Send-safe across
                                        // tokio futures), but for simplicity drain synchronously
                                        // in a blocking task.
                                        let key_for_thread = entry_key.clone();
                                        std::thread::spawn(move || {
                                            for line in &running.events_rx {
                                                if let Some(h) = RUNS.write().get_mut(&key_for_thread) {
                                                    h.events.push(line);
                                                }
                                            }
                                            if let Some(h) = RUNS.write().get_mut(&key_for_thread) {
                                                // Don't clobber a user Cancel/Fail that
                                                // landed while the stream was draining.
                                                if matches!(h.status, RunStatus::Running) {
                                                    h.status = RunStatus::Done;
                                                    h.finished_at = Some(Utc::now());
                                                    h.child_pid = None;
                                                }
                                            }
                                        });
                                    }
                                    Err(e) => {
                                        if let Some(h) = RUNS.write().get_mut(&entry_key) {
                                            h.status = RunStatus::Failed;
                                            h.finished_at = Some(Utc::now());
                                            h.events.push(format!("[spawn error] {e}"));
                                        }
                                    }
                                }
                            });
                            let _ = entry_key_for_close;
                        },
                        "▶ Start run"
                    }
                }
            }
        }
    }
}

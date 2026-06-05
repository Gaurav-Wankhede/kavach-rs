// L0 App shell — root component, hosts page routing + global modals.
// Live refresh is event-driven: ONE root long-poll loop (mounted once via
// use_future) awaits the daemon's change feed and bumps REFRESH_TICK whenever
// real data changes — the manual refresh button still works as a fallback.
// The prior auto-refresh loop crashed because it re-spawned per render; this
// one mounts exactly once (AppShell is the singular root) and never re-runs.
// SOURCE: https://dioxuslabs.com/learn/0.7/essentials/basics/async/
use dioxus::prelude::*;

use crate::molecules::entry_editor::EntryEditor;
use crate::molecules::run_modal::RunModal;
use crate::organisms::main_panel::MainPanel;
use crate::organisms::sidebar::Sidebar;
use crate::organisms::topbar::TopBar;
use crate::pages::{
    concepts::ConceptsPage, decisions::DecisionsPage, kanban::KanbanPage, knowledge::KnowledgePage,
    mistakes::MistakesPage, projects::ProjectsPage, roadmap::RoadmapPage, runs::RunsPage,
};
use crate::rpc_client::wait_for_change;
use crate::state::{ACTIVE_TAB, REFRESH_TICK, Tab};

/// Backoff applied after a failed long-poll (daemon offline / transient I/O)
/// before retrying, so a downed daemon doesn't spin the loop hot.
const POLL_BACKOFF: std::time::Duration = std::time::Duration::from_secs(2);

#[component]
pub fn AppShell() -> Element {
    // The single live-update loop for the whole app. use_future mounts this
    // exactly once; the loop never returns, so it runs for the app's lifetime.
    // Each blocking RPC runs on a blocking thread (spawn_blocking) so the
    // Dioxus reactor is never stalled. On a real change we bump REFRESH_TICK,
    // which every page's use_reactive!(tick) re-runs against — one signal,
    // every page refreshes. On error we back off and retry (fail-open: the
    // manual refresh button remains the user's escape hatch).
    use_future(move || async move {
        let mut seen: u64 = 0;
        loop {
            match tokio::task::spawn_blocking(move || wait_for_change(seen)).await {
                Ok(Ok(version)) => {
                    if version > seen {
                        seen = version;
                        // wrapping: a u64 refresh tick that wraps after 2^64
                        // bumps is harmless — pages react to any change, not
                        // the absolute value. Avoids the arithmetic-side-effects lint.
                        let next = REFRESH_TICK.peek().wrapping_add(1);
                        *REFRESH_TICK.write() = next;
                    }
                    // version == seen → idle timeout, just poll again (no bump).
                }
                Ok(Err(_)) | Err(_) => {
                    // Daemon offline / transient I/O / join error: back off,
                    // keep the loop alive so refresh resumes when it recovers.
                    tokio::time::sleep(POLL_BACKOFF).await;
                }
            }
        }
    });

    let tab = ACTIVE_TAB.read().clone();
    rsx! {
        style { {include_str!("../assets/app.css")} }
        div { class: "app-shell",
            TopBar {}
            div { class: "app-body",
                Sidebar {}
                MainPanel {
                    match tab {
                        Tab::Projects => rsx! { ProjectsPage {} },
                        Tab::Roadmap => rsx! { RoadmapPage {} },
                        Tab::Kanban => rsx! { KanbanPage {} },
                        Tab::Decisions => rsx! { DecisionsPage {} },
                        Tab::Knowledge => rsx! { KnowledgePage {} },
                        Tab::Runs => rsx! { RunsPage {} },
                        Tab::Concepts => rsx! { ConceptsPage {} },
                        Tab::Mistakes => rsx! { MistakesPage {} },
                    }
                }
            }
            EntryEditor {}
            RunModal {}
        }
    }
}

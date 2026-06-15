mod board;
mod dag_view;
mod data;
mod deps;
mod tiers;

use dioxus::prelude::*;

use crate::state::{REFRESH_TICK, SELECTED_PROJECT};
use board::KanbanBoard;
use dag_view::DagView;
use data::{LoadState, load};

/// Which lens the kanban page renders: the classic 4-status board, or the
/// dependency DAG grouped into topological tiers.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Status,
    Dag,
}

#[component]
pub(crate) fn KanbanPage() -> Element {
    let proj = SELECTED_PROJECT.read().clone();
    let tick: u64 = *REFRESH_TICK.read();
    let Some(slug) = proj else {
        return rsx! {
            section { class: "page page-kanban",
                h1 { "Kanban" }
                p { class: "hint", "Select a project." }
            }
        };
    };
    let rows = use_resource(use_reactive!(|slug, tick| async move {
        let _ = tick;
        load(&slug)
    }));
    let mut mode = use_signal(|| ViewMode::Status);
    let current = *mode.read();
    rsx! {
        section { class: "page page-kanban",
            div { class: "kanban-header",
                h1 { "Kanban" }
                div { class: "view-toggle",
                    button {
                        class: if current == ViewMode::Status { "toggle-btn active" } else { "toggle-btn" },
                        onclick: move |_| mode.set(ViewMode::Status),
                        "Status board"
                    }
                    button {
                        class: if current == ViewMode::Dag { "toggle-btn active" } else { "toggle-btn" },
                        onclick: move |_| mode.set(ViewMode::Dag),
                        "DAG (tiers)"
                    }
                }
            }
            match current {
                ViewMode::Status => rsx! { KanbanBoard { rows: rows } },
                ViewMode::Dag => {
                    let snap = rows.read_unchecked();
                    match &*snap {
                        None => rsx! { div { class: "skeleton", "Loading…" } },
                        Some(LoadState::DaemonOffline) => rsx! {
                            div { class: "banner banner-warn",
                                "kavach-rpc daemon offline — start via "
                                code { "kavach rpc serve" }
                            }
                        },
                        Some(LoadState::Ok(v)) => rsx! { DagView { rows: v.clone() } },
                    }
                }
            }
        }
    }
}

#![allow(
    clippy::same_name_method,
    reason = "dioxus #[component] macro generates builder() that collides with typed-builder trait"
)]
use dioxus::prelude::*;
use kavach_types::MemoryStatus;

use crate::molecules::entry_row::EntryRow;
use crate::pages::kanban::data::{LoadState, delete};
use crate::state::{EntryRef, REFRESH_TICK};

#[component]
pub(crate) fn KanbanBoard(rows: Resource<LoadState>) -> Element {
    let snap = rows.read_unchecked();
    let list = match &*snap {
        None => return rsx! { div { class: "skeleton", "Loading…" } },
        Some(LoadState::DaemonOffline) => {
            return rsx! {
                div { class: "banner banner-warn",
                    "kavach-rpc daemon offline — start via "
                    code { "kavach rpc serve" }
                }
            };
        }
        Some(LoadState::Ok(v)) => v.clone(),
    };
    rsx! {
        div { class: "kanban-board",
            for status in MemoryStatus::all() {
                KanbanColumn {
                    key: "{status}",
                    status,
                    rows: list.clone(),
                }
            }
        }
    }
}

#[component]
fn KanbanColumn(status: MemoryStatus, rows: Vec<EntryRef>) -> Element {
    rsx! {
        div { class: "kanban-col",
            h3 { "{status}" }
            for entry in rows.into_iter().filter(|r| r.status == status) {
                EntryRow {
                    key: "{entry.key}",
                    entry: entry,
                    updated_at: None,
                    links: Vec::new(),
                    on_delete: move |t: EntryRef| {
                        spawn(async move {
                            delete(&t);
                            REFRESH_TICK.with_mut(|tick| *tick = tick.wrapping_add(1));
                        });
                    },
                }
            }
        }
    }
}

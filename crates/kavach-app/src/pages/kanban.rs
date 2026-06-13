mod board;
mod data;
mod deps;

use dioxus::prelude::*;

use crate::state::{REFRESH_TICK, SELECTED_PROJECT};
use board::KanbanBoard;
use data::load;

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
    rsx! {
        section { class: "page page-kanban",
            h1 { "Kanban" }
            KanbanBoard { rows: rows }
        }
    }
}

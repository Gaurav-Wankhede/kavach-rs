pub mod data;
mod list;

use dioxus::prelude::*;

use crate::state::{REFRESH_TICK, SELECTED_PROJECT};
use data::load;
use list::DecisionsList;

#[component]
pub(crate) fn DecisionsPage() -> Element {
    let proj = SELECTED_PROJECT.read().clone();
    let tick: u64 = *REFRESH_TICK.read();
    let Some(slug) = proj else {
        return rsx! {
            section { class: "page page-decisions",
                h1 { "Decisions" }
                p { class: "hint", "Select a project." }
            }
        };
    };
    let rows = use_resource(use_reactive!(|slug, tick| async move {
        let _ = tick;
        load(&slug)
    }));
    rsx! {
        section { class: "page page-decisions",
            h1 { "Decisions" }
            DecisionsList { rows: rows }
        }
    }
}

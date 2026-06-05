pub mod data;
mod list;

use dioxus::prelude::*;

use crate::state::{REFRESH_TICK, SELECTED_PROJECT};
use data::load;
use list::RoadmapList;

#[component]
pub(crate) fn RoadmapPage() -> Element {
    let proj = SELECTED_PROJECT.read().clone();
    let tick: u64 = *REFRESH_TICK.read();
    let Some(slug) = proj else {
        return rsx! {
            section { class: "page page-roadmap",
                h1 { "Roadmap" }
                p { class: "hint", "Select a project." }
            }
        };
    };
    let rows = use_resource(use_reactive!(|slug, tick| async move {
        let _ = tick;
        load(&slug)
    }));
    rsx! {
        section { class: "page page-roadmap",
            h1 { "Roadmap" }
            RoadmapList { rows: rows }
        }
    }
}

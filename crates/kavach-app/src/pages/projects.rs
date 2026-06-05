// PAGE: Projects — landing page summarizing the selected project.
// SOURCE: https://dioxuslabs.com/learn/0.7
use dioxus::prelude::*;

use crate::state::SELECTED_PROJECT;

#[component]
pub(crate) fn ProjectsPage() -> Element {
    let proj = SELECTED_PROJECT.read().clone();
    rsx! {
        section { class: "page page-projects",
            h1 { "Projects" }
            {
                proj.map_or_else(
                    || rsx! { p { class: "hint", "Select a project from the sidebar." } },
                    |p| rsx! {
                        p { class: "selected", "Selected: " strong { "{p}" } }
                        p { class: "tip", "Switch tabs above to view roadmap, kanban, decisions, or knowledge graph." }
                    },
                )
            }
        }
    }
}

// ORGANISM L1: TopBar — title + tab strip + search + refresh
// SOURCE: https://dioxuslabs.com/learn/0.7
#![allow(
    clippy::same_name_method,
    reason = "dioxus #[component] macro generates builder() that collides with typed-builder trait"
)]
use dioxus::prelude::*;

use crate::state::{ACTIVE_TAB, REFRESH_TICK, SELECTED_PROJECT, Tab};

#[component]
pub fn TopBar() -> Element {
    let proj = SELECTED_PROJECT
        .read()
        .clone()
        .unwrap_or_else(|| String::from("(no project)"));
    let active = ACTIVE_TAB.read().clone();
    rsx! {
        header { class: "topbar",
            div { class: "topbar-brand", "kavach" }
            div { class: "topbar-project", "{proj}" }
            nav { class: "topbar-tabs",
                TabButton { tab: Tab::Projects, active: active.clone() }
                TabButton { tab: Tab::Roadmap, active: active.clone() }
                TabButton { tab: Tab::Kanban, active: active.clone() }
                TabButton { tab: Tab::Decisions, active: active.clone() }
                TabButton { tab: Tab::Knowledge, active: active.clone() }
                TabButton { tab: Tab::Runs, active: active.clone() }
                TabButton { tab: Tab::Concepts, active: active.clone() }
                TabButton { tab: Tab::Mistakes, active: active }
            }
            button {
                class: "topbar-refresh",
                title: "Refresh data",
                onclick: move |_| { *REFRESH_TICK.write() += 1; },
                "↻"
            }
        }
    }
}

#[component]
fn TabButton(tab: Tab, active: Tab) -> Element {
    let is_active = tab == active;
    let cls = if is_active { "tab tab-active" } else { "tab" };
    let label = tab.label();
    let target = tab.clone();
    rsx! {
        button {
            class: "{cls}",
            onclick: move |_| { *ACTIVE_TAB.write() = target.clone(); },
            "{label}"
        }
    }
}

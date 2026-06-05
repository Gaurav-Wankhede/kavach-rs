// ORGANISM L2: MainPanel — empty content slot. Page selection happens in AppShell.
// Organisms must not import pages (atomic-UI rule).
// SOURCE: https://dioxuslabs.com/learn/0.7
#![allow(
    clippy::same_name_method,
    reason = "dioxus #[component] macro generates builder() that collides with typed-builder trait"
)]
use dioxus::prelude::*;

#[component]
pub fn MainPanel(children: Element) -> Element {
    rsx! { main { class: "main-panel", {children} } }
}

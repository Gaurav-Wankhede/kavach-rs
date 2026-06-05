// ATOM: log tail view — monospace, scrollable, append-only line list.
// SOURCE: https://dioxuslabs.com/learn/0.7
#![allow(
    clippy::same_name_method,
    reason = "dioxus #[component] macro generates builder() that collides with typed-builder trait"
)]
use dioxus::prelude::*;

#[component]
pub fn LogView(lines: Vec<String>) -> Element {
    rsx! {
        pre { class: "log-view",
            for (i, line) in lines.iter().enumerate() {
                div { key: "{i}", class: "log-line", "{line}" }
            }
        }
    }
}

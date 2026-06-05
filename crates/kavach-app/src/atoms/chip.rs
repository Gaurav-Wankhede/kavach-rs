// ATOM: status chip
// SOURCE: https://dioxuslabs.com/learn/0.7
#![allow(
    clippy::same_name_method,
    reason = "dioxus #[component] macro generates builder() that collides with typed-builder trait"
)]
use dioxus::prelude::*;

fn classify(status: &str) -> &'static str {
    if status == "todo" {
        return "chip chip-todo";
    }
    if status == "in_progress" {
        return "chip chip-progress";
    }
    if status == "done" {
        return "chip chip-done";
    }
    if status == "verified" {
        return "chip chip-verified";
    }
    "chip"
}

#[component]
pub fn StatusChip(status: String) -> Element {
    let cls = classify(status.as_str());
    rsx! { span { class: "{cls}", "{status}" } }
}

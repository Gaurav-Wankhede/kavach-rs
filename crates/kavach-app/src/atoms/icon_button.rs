// ATOM: icon button — small button with single SVG-style glyph
// SOURCE: https://dioxuslabs.com/learn/0.7
#![allow(
    clippy::same_name_method,
    reason = "dioxus #[component] macro generates builder() that collides with typed-builder trait"
)]
use dioxus::prelude::*;

#[component]
pub fn IconButton(
    glyph: String,
    label: String,
    variant: String,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    let cls = if variant == "danger" {
        "icon-btn icon-btn-danger"
    } else if variant == "success" {
        "icon-btn icon-btn-success"
    } else if variant == "primary" {
        "icon-btn icon-btn-primary"
    } else if variant == "ghost" {
        "icon-btn icon-btn-ghost"
    } else {
        "icon-btn"
    };
    rsx! {
        button {
            class: "{cls}",
            "aria-label": "{label}",
            title: "{label}",
            onclick: move |e| onclick.call(e),
            "{glyph}"
        }
    }
}

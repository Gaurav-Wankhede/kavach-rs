// MOLECULE: tiny shell so the parent kanban can wire RUN_TARGET.
// Currently re-uses IconButton via entry_row; this file is reserved for
// future kanban-card placement where we want a bigger primary CTA.
// SOURCE: https://dioxuslabs.com/learn/0.7
#![allow(
    clippy::same_name_method,
    reason = "dioxus #[component] macro generates builder() that collides with typed-builder trait"
)]
use dioxus::prelude::*;

use crate::state::{EntryRef, RUN_TARGET};

#[component]
pub fn RunButton(entry: EntryRef) -> Element {
    let pick = entry;
    rsx! {
        button {
            class: "btn btn-success",
            onclick: move |_| { *RUN_TARGET.write() = Some(pick.clone()); },
            "▶ Run in Claude Code"
        }
    }
}

// ALGO: ModalControlledForm
// PROBLEM_CLASS: cache
// REJECTED: [{"name":"inline_editor","reason":"competes for row real estate"},{"name":"separate_route","reason":"loses list context"}]
// TIME: O(1) per keystroke (signal write) | SPACE: O(content size)
// YEAR: 2026 | SEARCHED: 2026-05
// TRADEOFF: blocks underlying list while open
// BENCHMARK: https://dioxuslabs.com/learn/0.7/essentials/basics/signals/
// SOURCE: https://dioxuslabs.com/learn/0.7
mod save;

use dioxus::prelude::*;
use kavach_types::MemoryStatus;

use crate::state::{EDITING_ENTRY, EntryRef, REFRESH_TICK, status_from_str};
use save::save;

#[component]
pub fn EntryEditor() -> Element {
    let snap = EDITING_ENTRY.read().clone();
    let Some(target) = snap else { return rsx! {} };
    let mut title = use_signal(|| target.title.clone());
    let mut content = use_signal(|| target.content.clone());
    let mut status = use_signal(|| target.status);
    let mut error = use_signal(String::new);
    let project_slug = target.project_slug.clone();
    let category = target.category.clone();
    let key = target.key;

    rsx! {
        div { class: "modal-backdrop", onclick: move |_| { *EDITING_ENTRY.write() = None; },
            div { class: "modal", onclick: move |e| e.stop_propagation(),
                h2 { "Edit {category} / {key}" }
                label { "Title"
                    input { class: "input", value: "{title}", oninput: move |e| title.set(e.value()) }
                }
                label { "Content"
                    textarea {
                        class: "textarea", rows: "10", value: "{content}",
                        oninput: move |e| content.set(e.value()),
                    }
                }
                label { "Status"
                    select {
                        class: "select", value: "{status}",
                        oninput: move |e| status.set(status_from_str(&e.value())),
                        for opt in MemoryStatus::all() {
                            option { value: "{opt}", selected: *status.read() == opt, "{opt}" }
                        }
                    }
                }
                if !error.read().is_empty() { div { class: "modal-error", "{error}" } }
                div { class: "modal-actions",
                    button {
                        class: "btn",
                        onclick: move |_| { *EDITING_ENTRY.write() = None; },
                        "Cancel"
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            let t = EntryRef {
                                project_slug: project_slug.clone(),
                                category: category.clone(),
                                key: key.clone(),
                                title: title.read().clone(),
                                content: content.read().clone(),
                                status: *status.read(),
                            };
                            spawn(async move {
                                match save(&t) {
                                    Ok(()) => {
                                        *EDITING_ENTRY.write() = None;
                                        *REFRESH_TICK.write() += 1;
                                    }
                                    Err(e) => error.set(e),
                                }
                            });
                        },
                        "Save"
                    }
                }
            }
        }
    }
}

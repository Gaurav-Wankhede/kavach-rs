use dioxus::prelude::*;

use crate::pages::concepts::data::{LoadState, add_concept, list_concepts, search_concepts};
use crate::state::REFRESH_TICK;

#[component]
pub fn ConceptsView() -> Element {
    let mut query = use_signal(String::new);
    let mut new_name = use_signal(String::new);
    let mut new_display = use_signal(String::new);
    let mut new_desc = use_signal(String::new);
    let mut new_source = use_signal(String::new);
    let mut add_error = use_signal(String::new);
    let tick: u64 = *REFRESH_TICK.read();
    let q = query.read().clone();
    let concepts = use_resource(use_reactive!(|q, tick| async move {
        let _ = tick;
        if q.trim().is_empty() {
            list_concepts(50)
        } else {
            search_concepts(&q, 20)
        }
    }));
    let snap = concepts.read_unchecked();
    rsx! {
        section { class: "page page-concepts",
            h1 { "Concepts" }
            div { class: "concept-search",
                input {
                    class: "input",
                    placeholder: "Search concepts (BM25)",
                    value: "{query}",
                    oninput: move |e| query.set(e.value()),
                }
            }
            match &*snap {
                None => rsx! { div { class: "skeleton", "Loading…" } },
                Some(LoadState::DaemonOffline) => rsx! {
                    div { class: "banner banner-warn",
                        "kavach-rpc daemon offline — start via "
                        code { "kavach rpc serve" }
                    }
                },
                Some(LoadState::Ok(list)) if list.is_empty() => {
                    rsx! { div { class: "empty", "No concepts." } }
                }
                Some(LoadState::Ok(list)) => rsx! {
                    ul { class: "concept-list",
                        for c in list.clone() {
                            li { class: "concept-row",
                                strong { "{c.name}" }
                                span { class: "concept-type", " · {c.entity_type}" }
                            }
                        }
                    }
                },
            }
            details { class: "concept-add",
                summary { "Add concept (source URL required)" }
                input { class: "input", placeholder: "name", value: "{new_name}",
                    oninput: move |e| new_name.set(e.value()) }
                input { class: "input", placeholder: "display", value: "{new_display}",
                    oninput: move |e| new_display.set(e.value()) }
                textarea { class: "textarea", rows: "3", placeholder: "description",
                    value: "{new_desc}", oninput: move |e| new_desc.set(e.value()) }
                input { class: "input", placeholder: "source URL (https://...)",
                    value: "{new_source}", oninput: move |e| new_source.set(e.value()) }
                if !add_error.read().is_empty() {
                    div { class: "modal-error", "{add_error}" }
                }
                button {
                    class: "btn btn-primary",
                    onclick: move |_| {
                        let n = new_name.read().clone();
                        let d = new_display.read().clone();
                        let desc = new_desc.read().clone();
                        let src = new_source.read().clone();
                        spawn(async move {
                            match add_concept(&n, &d, &desc, &src) {
                                Ok(()) => {
                                    new_name.set(String::new());
                                    new_display.set(String::new());
                                    new_desc.set(String::new());
                                    new_source.set(String::new());
                                    add_error.set(String::new());
                                    REFRESH_TICK.with_mut(|prev| *prev = prev.wrapping_add(1));
                                }
                                Err(e) => add_error.set(e),
                            }
                        });
                    },
                    "Add"
                }
            }
        }
    }
}

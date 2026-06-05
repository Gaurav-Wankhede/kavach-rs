use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::rpc_client::{Error as RpcError, rpc};

#[derive(Debug, Serialize)]
struct HitCountParams<'a> {
    name: &'a str,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
struct HitCountResultDto {
    name: String,
    hit_count: i64,
}

#[derive(Clone, PartialEq)]
enum LookupState {
    Idle,
    Ok(HitCountResultDto),
    DaemonOffline,
    Err(String),
}

#[component]
pub(crate) fn MistakesPage() -> Element {
    let mut name_input = use_signal(String::new);
    let mut state = use_signal(|| LookupState::Idle);
    rsx! {
        section { class: "page page-mistakes",
            h1 { "Mistakes" }
            p { class: "hint",
                "Lookup hit count for a specific anti-pattern (e.g. "
                code { "anti.rust-guard.a1b2c3d4" } "). A top-N listing RPC will land in a follow-up unit."
            }
            div { class: "mistake-search",
                input {
                    class: "input",
                    placeholder: "anti-pattern name",
                    value: "{name_input}",
                    oninput: move |e| name_input.set(e.value()),
                }
                button {
                    class: "btn btn-primary",
                    onclick: move |_| {
                        let n = name_input.read().clone();
                        spawn(async move {
                            let res = rpc::<HitCountParams<'_>, HitCountResultDto>(
                                "mistake.hit_count",
                                HitCountParams { name: &n },
                            );
                            let next = match res {
                                Ok(v) => LookupState::Ok(v),
                                Err(RpcError::DaemonOffline(_)) => LookupState::DaemonOffline,
                                Err(e) => LookupState::Err(e.to_string()),
                            };
                            state.set(next);
                        });
                    },
                    "Lookup"
                }
            }
            match &*state.read() {
                LookupState::Idle => rsx! { div { class: "empty", "Enter an anti-pattern name." } },
                LookupState::DaemonOffline => rsx! {
                    div { class: "banner banner-warn",
                        "kavach-rpc daemon offline — start via "
                        code { "kavach rpc serve" }
                    }
                },
                LookupState::Err(e) => rsx! { div { class: "modal-error", "{e}" } },
                LookupState::Ok(r) => rsx! {
                    div { class: "mistake-row",
                        strong { "{r.name}" }
                        span { class: "mistake-count", " · hits={r.hit_count}" }
                    }
                },
            }
        }
    }
}

#![allow(
    clippy::same_name_method,
    reason = "dioxus #[component] macro generates builder() that collides with typed-builder trait"
)]
use dioxus::prelude::*;

use crate::molecules::entry_row::EntryRow;
use crate::pages::decisions::data::{LoadState, delete};
use crate::state::{EntryRef, REFRESH_TICK};

#[component]
pub(crate) fn DecisionsList(rows: Resource<LoadState>) -> Element {
    let snap = rows.read_unchecked();
    match &*snap {
        None => rsx! { div { class: "skeleton", "Loading…" } },
        Some(LoadState::DaemonOffline) => rsx! {
            div { class: "banner banner-warn",
                "kavach-rpc daemon offline — start via "
                code { "kavach rpc serve" }
            }
        },
        Some(LoadState::Ok(list)) if list.is_empty() => {
            rsx! { div { class: "empty", "No decisions." } }
        }
        Some(LoadState::Ok(list)) => rsx! {
            div { class: "entry-table",
                for entry in list.clone() {
                    EntryRow {
                        key: "{entry.key}",
                        entry: entry,
                        updated_at: None,
                        links: Vec::new(),
                        on_delete: move |t: EntryRef| {
                            spawn(async move {
                                delete(&t);
                                REFRESH_TICK.with_mut(|tick| *tick = tick.wrapping_add(1));
                            });
                        },
                    }
                }
            }
        },
    }
}

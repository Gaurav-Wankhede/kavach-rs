#![allow(
    clippy::same_name_method,
    reason = "dioxus #[component] macro generates builder() that collides with typed-builder trait"
)]
//! Tier-grouped DAG view — the desktop visualization of the dependency graph,
//! the GUI counterpart of `kavach db kanban`'s tiered text. Cards are grouped by
//! topological depth (TIER 0 = ready now, TIER N unlocks once tier `<N` closes)
//! instead of by status, with each card's on-board prerequisites shown inline.
//! Cards on a dependency cycle are rendered apart, never silently dropped.
use dioxus::prelude::*;

use crate::molecules::entry_row::EntryRow;
use crate::pages::kanban::data::delete;
use crate::pages::kanban::tiers::layout;
use crate::state::{EntryRef, REFRESH_TICK};

#[component]
pub(crate) fn DagView(rows: Vec<EntryRef>) -> Element {
    let (tiers, cyclic) = layout(&rows);
    rsx! {
        div { class: "kanban-dag",
            for (tier_idx , nodes) in tiers.iter().enumerate() {
                if !nodes.is_empty() {
                    div { class: "dag-tier",
                        h3 { class: "dag-tier-head",
                            if tier_idx == 0 {
                                "TIER 0 — ready now"
                            } else {
                                "TIER {tier_idx}"
                            }
                        }
                        div { class: "dag-tier-cards",
                            for node in nodes {
                                DagCard {
                                    key: "{node.entry.key}",
                                    entry: node.entry.clone(),
                                    deps: node.deps.clone(),
                                }
                            }
                        }
                    }
                }
            }
            if !cyclic.is_empty() {
                div { class: "dag-tier dag-cycle",
                    h3 { class: "dag-tier-head dag-cycle-head",
                        "⚠ CYCLE — these cards depend on each other and can never dispatch"
                    }
                    div { class: "dag-tier-cards",
                        for entry in cyclic {
                            DagCard {
                                key: "{entry.key}",
                                entry,
                                deps: Vec::new(),
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn DagCard(entry: EntryRef, deps: Vec<String>) -> Element {
    rsx! {
        div { class: "kanban-card-wrap",
            EntryRow {
                key: "{entry.key}",
                entry,
                updated_at: None,
                links: Vec::new(),
                on_delete: move |t: EntryRef| {
                    spawn(async move {
                        delete(&t);
                        REFRESH_TICK.with_mut(|tick| *tick = tick.wrapping_add(1));
                    });
                },
            }
            if !deps.is_empty() {
                div { class: "dag-deps", "⤷ depends-on: {deps.join(\", \")}" }
            }
        }
    }
}

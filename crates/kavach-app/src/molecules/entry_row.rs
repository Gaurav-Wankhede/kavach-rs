// MOLECULE: entry row — chip + key + title + ts + link badges + actions + disclosure.
// SOURCE: https://dioxuslabs.com/learn/0.7
// SOURCE: https://www.w3.org/WAI/ARIA/apg/patterns/disclosure/
#![allow(
    clippy::same_name_method,
    reason = "dioxus #[component] macro generates builder() that collides with typed-builder trait"
)]
use chrono::{DateTime, Utc};
use dioxus::prelude::*;

use crate::atoms::chip::StatusChip;
use crate::atoms::icon_button::IconButton;
use crate::atoms::relative_time::RelativeTime;
use crate::state::{EDITING_ENTRY, EntryRef, LinkSummary, RUN_TARGET};

#[component]
pub fn EntryRow(
    entry: EntryRef,
    updated_at: Option<DateTime<Utc>>,
    on_delete: EventHandler<EntryRef>,
    #[props(default)] links: Vec<LinkSummary>,
) -> Element {
    let mut expanded = use_signal(|| false);
    let mut confirming_delete = use_signal(|| false);
    let mut delete_phrase = use_signal(String::new);
    let edit_target = entry.clone();
    let run_target = entry.clone();
    let delete_target = entry.clone();
    // Target-bound confirmation the daemon requires; mirrors
    // kavach_rpc::methods::db::delete_confirm_phrase (single-key form). The user
    // must type this exact string to arm the delete — no auto-confirm.
    let expected_phrase = format!(
        "delete {}/{}/{}",
        entry.project_slug, entry.category, entry.key
    );
    let phrase_matches = *delete_phrase.read() == expected_phrase;
    // FIX [contract_violation]: align panel id + class names with canonical
    // schema field `content`. SOURCE: nuxt-content/nuxt-studio#400.
    let panel_id = format!("content-{}-{}", entry.category, entry.key);
    let panel_id_for_button = panel_id.clone();
    let content_for_panel = entry.content.clone();
    let is_open = *expanded.read();
    let toggle_glyph = if is_open {
        String::from("▼")
    } else {
        String::from("▶")
    };
    let toggle_label = if is_open {
        String::from("Hide details")
    } else {
        String::from("Show details")
    };

    let out_count = links.iter().filter(|l| l.direction == "out").count();
    let in_count = links.iter().filter(|l| l.direction == "in").count();
    let has_links = !links.is_empty();

    rsx! {
        div { class: "entry-row-wrap",
            div { class: "entry-row",
                StatusChip { status: entry.status }
                span { class: "entry-key", "{entry.key}" }
                span { class: "entry-title", "{entry.title}" }
                RelativeTime { timestamp: updated_at }
                if has_links {
                    div { class: "entry-links-badge",
                        title: "{out_count} outbound · {in_count} inbound",
                        if out_count > 0 { span { class: "link-badge link-out", "→{out_count}" } }
                        if in_count > 0  { span { class: "link-badge link-in",  "←{in_count}" } }
                    }
                }
                div { class: "entry-actions",
                    button {
                        class: "icon-btn icon-btn-ghost",
                        "aria-label": "{toggle_label}",
                        "aria-expanded": if is_open { "true" } else { "false" },
                        "aria-controls": "{panel_id_for_button}",
                        title: "{toggle_label}",
                        onclick: move |_| {
                            let next = !*expanded.read();
                            expanded.set(next);
                        },
                        "{toggle_glyph}"
                    }
                    IconButton {
                        glyph: "▶".to_owned(),
                        label: "Run in Claude Code".to_owned(),
                        variant: "success".to_owned(),
                        onclick: move |_| { *RUN_TARGET.write() = Some(run_target.clone()); },
                    }
                    IconButton {
                        glyph: "✎".to_owned(),
                        label: "Edit".to_owned(),
                        variant: "primary".to_owned(),
                        onclick: move |_| { *EDITING_ENTRY.write() = Some(edit_target.clone()); },
                    }
                    IconButton {
                        glyph: "🗑".to_owned(),
                        label: "Delete".to_owned(),
                        variant: "danger".to_owned(),
                        onclick: move |_| {
                            let next = !*confirming_delete.read();
                            confirming_delete.set(next);
                            if !next { delete_phrase.set(String::new()); }
                        },
                    }
                }
            }
            if *confirming_delete.read() {
                div { class: "entry-delete-confirm", role: "alertdialog",
                    span { class: "entry-delete-prompt",
                        "To delete, type: "
                        code { "{expected_phrase}" }
                    }
                    input {
                        class: "input",
                        "aria-label": "Type the confirmation phrase to delete",
                        placeholder: "{expected_phrase}",
                        value: "{delete_phrase}",
                        oninput: move |e| delete_phrase.set(e.value()),
                    }
                    button {
                        class: "btn",
                        onclick: move |_| {
                            confirming_delete.set(false);
                            delete_phrase.set(String::new());
                        },
                        "Cancel"
                    }
                    button {
                        class: "btn btn-danger",
                        disabled: !phrase_matches,
                        onclick: move |_| {
                            if phrase_matches {
                                on_delete.call(delete_target.clone());
                                confirming_delete.set(false);
                                delete_phrase.set(String::new());
                            }
                        },
                        "Delete"
                    }
                }
            }
            if is_open {
                div { id: "{panel_id}", class: "entry-content", role: "region",
                    if content_for_panel.trim().is_empty() {
                        em { class: "entry-content-empty", "(no content)" }
                    } else {
                        pre { class: "entry-content-body", "{content_for_panel}" }
                    }
                    if has_links {
                        div { class: "entry-links-list",
                            strong { "Graph links" }
                            for link in links.iter() {
                                div { class: "link-item",
                                    span { class: "link-rel link-rel-{link.direction}", "{link.direction} {link.rel}" }
                                    span { class: "link-target", "{link.target_qname}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

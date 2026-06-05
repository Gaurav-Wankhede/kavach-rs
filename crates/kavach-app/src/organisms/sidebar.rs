#![allow(
    clippy::same_name_method,
    reason = "dioxus #[component] macro generates builder() that collides with typed-builder trait"
)]
use dioxus::prelude::*;
use serde::Deserialize;

use crate::rpc_client::rpc_no_params;
use crate::state::SELECTED_PROJECT;

#[derive(Debug, Deserialize)]
struct ProjectDto {
    slug: String,
}

enum FetchResult {
    Ok(Vec<String>),
    DaemonOffline,
}

fn fetch_projects() -> FetchResult {
    match rpc_no_params::<Vec<ProjectDto>>("projects.list_all") {
        Ok(rows) => FetchResult::Ok(rows.into_iter().map(|p| p.slug).collect()),
        Err(e) if e.is_daemon_offline() => FetchResult::DaemonOffline,
        Err(e) => {
            tracing::error!(error = %e, "projects.list_all failed");
            FetchResult::Ok(Vec::new())
        }
    }
}

#[component]
pub fn Sidebar() -> Element {
    let projects = use_resource(|| async { fetch_projects() });
    rsx! {
        aside { class: "sidebar",
            h2 { "Projects" }
            match &*projects.read_unchecked() {
                None => rsx! { div { class: "skeleton", "Loading…" } },
                Some(FetchResult::DaemonOffline) => rsx! {
                    div { class: "banner banner-warn",
                        "kavach-rpc daemon offline — start via "
                        code { "kavach rpc serve" }
                    }
                },
                Some(FetchResult::Ok(list)) if list.is_empty() => rsx! { div { class: "empty", "No projects" } },
                Some(FetchResult::Ok(list)) => rsx! {
                    ul { class: "project-list",
                        for slug in list.clone() {
                            ProjectListItem { slug: slug }
                        }
                    }
                },
            }
        }
    }
}

#[component]
fn ProjectListItem(slug: String) -> Element {
    let pick = slug.clone();
    let label = slug;
    rsx! {
        li {
            button {
                class: "project-item",
                onclick: move |_| { *SELECTED_PROJECT.write() = Some(pick.clone()); },
                "{label}"
            }
        }
    }
}

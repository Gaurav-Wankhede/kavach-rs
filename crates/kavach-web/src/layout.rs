//! Shared page chrome: the maud base shell + sidebar nav.
//!
//! Every full-page handler wraps its body in [`shell`]. HTMX (vendored under
//! `/static`) drives fragment swaps; an SSE connection to `/events` emits a
//! `refresh` event whenever the daemon's change feed advances, and any element
//! carrying `hx-trigger="sse:refresh"` re-fetches itself — the server-side
//! replacement for the old Dioxus `REFRESH_TICK` signal.

use maud::{DOCTYPE, Markup, html};

/// The sidebar tabs: (path, label). Mirrors the deleted GUI's tab set.
const NAV: &[(&str, &str)] = &[
    ("/", "Projects"),
    ("/roadmap", "Roadmap"),
    ("/kanban", "Kanban"),
    ("/decisions", "Decisions"),
    ("/knowledge", "Knowledge"),
    ("/concepts", "Concepts"),
    ("/citations", "Citations"),
    ("/mistakes", "Mistakes"),
    ("/runs", "Runs"),
];

/// Wrap `body` in the full HTML document with sidebar. `active` is the path of
/// the current page so the matching nav link is highlighted.
#[must_use]
pub fn shell(active: &str, project: Option<&str>, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "Kavach" }
                link rel="stylesheet" href="/static/app.css";
                script src="/static/htmx.min.js" {}
                script src="/static/sse.js" {}
            }
            body hx-ext="sse" sse-connect="/events" {
                aside.sidebar {
                    div.brand { "🛡 Kavach" }
                    @if let Some(p) = project {
                        div.project-pill { (p) }
                    }
                    nav {
                        @for (path, label) in NAV {
                            a.nav-link.active[*path == active] href=(path) { (label) }
                        }
                    }
                }
                main.content {
                    (body)
                }
            }
        }
    }
}

/// A standard page heading row.
#[must_use]
pub fn heading(title: &str) -> Markup {
    html! { header.page-head { h1 { (title) } } }
}

/// Wrap a page's data section in a live container that re-fetches `fragment_url`
/// on initial load and whenever the SSE `refresh` event fires. `inner` is the
/// server-rendered first paint so the page is useful even before HTMX boots.
#[must_use]
pub fn live(fragment_url: &str, inner: Markup) -> Markup {
    html! {
        div hx-get=(fragment_url) hx-trigger="sse:refresh from:body" hx-swap="innerHTML" {
            (inner)
        }
    }
}

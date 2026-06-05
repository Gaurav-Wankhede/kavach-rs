// Entry point for the kavach desktop client.
// SOURCE: https://dioxuslabs.com/learn/0.7/guides/platforms/
// REASON: Dioxus rsx! emits fully-qualified `dioxus_elements::elements::*` paths
// that the unused_qualifications lint flags at every event-attr site.
// SOURCE: https://deepwiki.com/DioxusLabs/dioxus/4.1-rsx-syntax-and-macro-system
#![allow(
    unused_qualifications,
    reason = "dioxus rsx macro emits fully-qualified paths"
)]
// REASON: #[component]/Signal::global expand to pub items that satisfy the
// framework re-export contract but trip unreachable_pub at user sites.
// SOURCE: https://docs.rs/dioxus-core-macro/latest/dioxus_core_macro/macro.rsx.html
#![allow(
    unreachable_pub,
    reason = "dioxus #[component] and Signal::global expand to pub items"
)]

mod app;
mod atoms;
mod molecules;
mod organisms;
mod pages;
mod rpc_client;
mod runner;
mod state;

fn main() {
    tracing_subscriber::fmt::init();
    dioxus::launch(app::AppShell);
}

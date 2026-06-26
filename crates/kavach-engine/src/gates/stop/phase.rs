//! Phase-group guards: iteration-completion + kanban-card/status gates. Each is
//! its own single-responsibility child; this hub re-exports their `check` fns.

mod autonomous_gate;
mod foreign_tree;
mod iteration;
mod kanban_card;
mod kanban_status;
mod user_focus;

pub(crate) use autonomous_gate::check as autonomous_gate;
pub(crate) use foreign_tree::check as foreign_tree;
pub(crate) use iteration::check as iteration;
pub(crate) use kanban_card::check as kanban_card;
pub(crate) use kanban_status::check as kanban_status;
pub(crate) use user_focus::check as user_focus;

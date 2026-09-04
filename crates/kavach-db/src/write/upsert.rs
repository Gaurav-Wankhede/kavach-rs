// SOURCE: micro-file hub — roadmap.upsert-microfile-split; children own one fn each
mod entry;
mod full;
mod graph_stmts;
mod with_event;

pub use entry::upsert_entry;
pub use full::upsert_entry_full;
pub use with_event::upsert_entry_with_event;

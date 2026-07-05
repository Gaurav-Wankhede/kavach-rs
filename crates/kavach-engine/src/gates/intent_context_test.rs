//! `intent_context` tests, split by family: directives vs db-query injection.
#[path = "intent_context/tests/directives.rs"]
mod directives;
#[path = "intent_context/tests/db_query.rs"]
mod db_query;
#[path = "intent_context/tests/dispatch.rs"]
mod dispatch;
#[path = "intent_context/tests/dynamic.rs"]
mod dynamic;

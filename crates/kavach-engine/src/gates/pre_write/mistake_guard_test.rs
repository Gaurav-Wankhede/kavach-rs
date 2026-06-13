// Proves the [MISTAKE_GUARD] pre-retrieval guards: an empty or too-thin write
// is skipped BEFORE any RPC (embedding a one-liner is pure noise), so these
// cases are deterministic regardless of whether the daemon is running. The
// cosine ranking + floor itself is proven in kavach-surreal's nearest_test.rs;
// the live wiring is proven by the deploy smoke.
use super::{MIN_QUERY_LEN, advisory};
use crate::gates::pre_write_context::WriteContext;

// Minimal WriteContext for the advisory — only content / effective_content
// drive the guard; the rest are inert categorization flags.
fn ctx<'a>(content: &'a str, effective: &str) -> WriteContext<'a> {
    WriteContext {
        file_path: "src/x.rs",
        tool_name: "Write",
        content,
        effective_content: effective.to_owned(),
        is_code: true,
        is_test: false,
        is_rust: true,
        is_frontend: false,
    }
}

#[test]
fn empty_write_yields_no_advisory() {
    assert!(
        advisory(&ctx("", "")).is_none(),
        "an empty write has nothing to relevance-match"
    );
}

#[test]
fn too_short_query_skips_retrieval() {
    let short = "fn x() {}";
    assert!(
        short.len() < MIN_QUERY_LEN,
        "fixture must be under the floor"
    );
    assert!(
        advisory(&ctx(short, "")).is_none(),
        "a sub-floor query must skip retrieval before any RPC"
    );
}

#[test]
fn whitespace_only_query_skips_retrieval() {
    assert!(
        advisory(&ctx("   \n\t  ", "")).is_none(),
        "whitespace trims to empty ⇒ no retrieval"
    );
}

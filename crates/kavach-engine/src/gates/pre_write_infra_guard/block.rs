//! P0 block: GraphQL introspection enabled in prod, or schema without a depth
//! limit (nested-query `DoS`).
use super::super::platform_guard_msg::build_block;
use super::super::platform_guard_paths::is_test;
use super::{ENABLED, GQL, INTRO, is_infra_file};

/// `Some(reason)` on a GraphQL hardening P0 violation.
pub(crate) fn check(file_path: &str, content: &str) -> Option<String> {
    if !is_infra_file(file_path) || is_test(file_path) {
        return None;
    }
    let lc = content.to_lowercase();
    let mut p0: Vec<(&str, &str)> = Vec::new();

    if lc.contains(GQL)
        && lc.contains(INTRO)
        && (lc.contains("true") || lc.contains(ENABLED))
        && !lc.contains("development")
        && !lc.contains("dev_only")
    {
        p0.push((
            "GRAPHQL_INTROSPECTION_ENABLED",
            "Disable GraphQL introspection in production — it leaks the full schema to attackers.",
        ));
    }
    if lc.contains(GQL)
        && lc.contains("schema")
        && !lc.contains("depthlimit")
        && !lc.contains("depth_limit")
        && !lc.contains("maxdepth")
        && !lc.contains("max_depth")
        && !lc.contains("dev_only")
        && !lc.contains("development")
    {
        p0.push((
            "GRAPHQL_NO_DEPTH_LIMIT",
            "Add depth limiting to GraphQL schema (max 10-15) — prevents nested query DoS.",
        ));
    }
    if p0.is_empty() {
        return None;
    }
    Some(build_block("INFRA_PROTOCOL_GUARD", &p0))
}

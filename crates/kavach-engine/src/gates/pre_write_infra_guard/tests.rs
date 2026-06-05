//! GraphQL introspection/depth blocks, dev-mode + non-infra + test skips, and
//! long-poll / queue-idempotency advisories.
use super::{check, format_advisory};

#[test]
fn should_block_graphql_introspection_enabled() {
    assert!(
        check(
            "src/gateway.ts",
            "const schema = createSchema({ graphql: true, introspection: true })"
        )
        .is_some()
    );
}

#[test]
fn should_allow_graphql_introspection_in_dev() {
    assert!(
        check(
            "src/gateway.ts",
            "const schema = createSchema({ graphql: true, introspection: true, dev_only: true })"
        )
        .is_none()
    );
}

#[test]
fn should_block_graphql_schema_without_depth_limit() {
    assert!(
        check(
            "src/gateway.ts",
            "const graphql = buildSchema({ schema: typeDefs })"
        )
        .is_some()
    );
}

#[test]
fn should_allow_graphql_schema_with_depth_limit() {
    assert!(
        check(
            "src/gateway.ts",
            "const graphql = buildSchema({ schema: typeDefs, maxDepth: 15 })"
        )
        .is_none()
    );
}

#[test]
fn should_skip_non_infra_files() {
    assert!(check("src/styles.css", "graphql introspection true").is_none());
}

#[test]
fn should_skip_test_files() {
    assert!(check("src/gateway.test.ts", "graphql introspection true schema").is_none());
}

#[test]
fn should_advise_long_polling() {
    assert!(format_advisory("src/api.ts", "const longPoll = startPolling()").is_some());
}

#[test]
fn should_advise_queue_without_idempotency() {
    assert!(
        format_advisory(
            "src/consumer.ts",
            "export default { async queue(batch) { for (const msg of batch) {} } }"
        )
        .is_some()
    );
}

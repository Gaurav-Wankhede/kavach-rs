//! mod.rs P0 hard-block tests + the `// hub:` escape hatch.
use super::super::check;

#[test]
fn should_block_fn_body_in_mod_rs() {
    let msg = check(
        "src/services/mod.rs",
        "pub mod auth;\npub use auth::AuthService;\npub fn helper() { do_thing(); }",
    );
    assert!(msg.is_some());
    assert!(msg.unwrap().contains("LOGIC_IN_MOD"));
}

#[test]
fn should_block_async_fn_in_mod_rs() {
    let msg = check(
        "src/services/mod.rs",
        "pub mod users;\npub async fn handle() { }",
    );
    assert!(msg.is_some());
    assert!(msg.unwrap().contains("LOGIC_IN_MOD"));
}

#[test]
fn should_block_struct_in_mod_rs() {
    let msg = check(
        "src/services/mod.rs",
        "pub mod auth;\npub struct ServiceConfig { pub timeout: u64 }",
    );
    assert!(msg.is_some());
    assert!(msg.unwrap().contains("STRUCT_IN_MOD"));
}

#[test]
fn should_block_impl_in_mod_rs() {
    let msg = check(
        "src/services/mod.rs",
        "pub mod auth;\nimpl Default for Foo { fn default() -> Self { Self } }",
    );
    assert!(msg.is_some());
    assert!(msg.unwrap().contains("STRUCT_IN_MOD"));
}

#[test]
fn should_block_mod_rs_over_100_lines() {
    let content = "pub mod a;\n".repeat(101);
    let msg = check("src/services/mod.rs", &content);
    assert!(msg.is_some());
    assert!(msg.unwrap().contains("MOD_TOO_LARGE"));
}

#[test]
fn should_allow_clean_mod_rs_hub() {
    let content =
        "pub mod auth;\npub mod users;\npub use auth::AuthService;\nuse crate::error::Error;\n";
    assert!(check("src/services/mod.rs", content).is_none());
}

#[test]
fn should_allow_hub_marker_bypass_for_struct_in_mod_rs() {
    let content = "// hub: intentional — legacy layout\npub mod auth;\npub struct LegacyConfig {}";
    assert!(check("src/services/mod.rs", content).is_none());
}

#[test]
fn should_not_block_mod_rs_with_mixed_concerns_rule() {
    // mod.rs is covered by LOGIC_IN_MOD / STRUCT_IN_MOD, not FILE_MIXED_CONCERNS.
    let body = "pub struct Svc {}\nimpl Svc {}\npub async fn handler() {}\n".repeat(70);
    if let Some(m) = check("src/services/mod.rs", &body) {
        assert!(!m.contains("FILE_MIXED_CONCERNS"));
    }
}

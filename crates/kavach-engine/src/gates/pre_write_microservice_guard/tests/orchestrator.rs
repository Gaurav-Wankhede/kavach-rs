//! Orchestrator-rule + file-type-skip tests.
use super::super::check;

#[test]
fn should_block_state_struct_in_app_rs() {
    assert!(
        check(
            "src/app.rs",
            "pub struct AppState { pub fn new() {} arc<Pool> pool }"
        )
        .is_some()
    );
}

#[test]
fn should_allow_state_struct_in_service_module() {
    assert!(
        check(
            "src/media/state.rs",
            "pub struct MediaState { pub fn new() {} arc<Pool> pool }"
        )
        .is_none()
    );
}

#[test]
fn should_block_inline_init_in_main_rs() {
    assert!(
        check(
            "src/main.rs",
            "let svc = MyService::from_env(); let r = Router::new().route(\"/\", get(handler))"
        )
        .is_some()
    );
}

#[test]
fn should_allow_scattered_patterns_in_large_app_rs() {
    // Patterns exist but are far apart (>500 chars) — should NOT block.
    let mut content = String::from("let config = Config::from_env();\n");
    content.push_str(&"// filler line\n".repeat(50));
    content.push_str("let router = Router::new();\n");
    content.push_str(&"// more filler\n".repeat(50));
    content.push_str(".route(\"/api\", handler)\n");
    assert!(check("src/app.rs", &content).is_none());
}

#[test]
fn should_skip_non_rs_files() {
    assert!(
        check(
            "src/pages/feed.tsx",
            "pub struct AppState { arc<Pool> pool }"
        )
        .is_none()
    );
}

#[test]
fn should_skip_test_files() {
    assert!(check("src/app.test.rs", "pub struct AppState { pub fn new() {} }").is_none());
}

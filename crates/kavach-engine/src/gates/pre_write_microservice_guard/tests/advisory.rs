//! `format_advisory` P1 tests.
use super::super::format_advisory;

#[test]
fn should_advise_mixed_concerns_in_service_file() {
    assert!(
        format_advisory(
            "src/media/proxy.rs",
            "pub struct State {} impl State {} pub async fn handler() {} axum::Router"
        )
        .is_some()
    );
}

#[test]
fn should_not_advise_clean_handler_file() {
    assert!(
        format_advisory(
            "src/media/handler.rs",
            "pub async fn media_handler(State(s): State<S>) -> impl IntoResponse {}"
        )
        .is_none()
    );
}

#[test]
fn should_not_advise_mod_rs_advisory_only_blocks() {
    // mod.rs violations are P0 blocks, not advisories.
    assert!(format_advisory("src/services/mod.rs", "pub async fn handler() {}").is_none());
}

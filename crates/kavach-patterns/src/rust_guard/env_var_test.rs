//! Regression suite for the `§CENTRALIZED_CONFIG` env-var detector. The
//! `false_positive_set_is_empty` test discharges the engine-CLAUDE.md RULE that a
//! P0 promotion must ship a test proving the false-positive bound: every sanctioned
//! reader (fragment, dotenvy loader, `main.rs`, startup validator) and every
//! ungoverned path must produce ZERO hits, so a hit is always a real violation.
use super::*;

fn hits(path: &str, src: &str) -> usize {
    let mut v = Vec::new();
    scan(path, src, &mut v);
    v.len()
}

fn severity_of(path: &str, src: &str) -> Option<Severity> {
    let mut v = Vec::new();
    scan(path, src, &mut v);
    v.first().map(|hit| hit.severity)
}

#[test]
fn flags_raw_env_var_in_governed_handler() {
    let n = hits(
        "/x/crates/core/payment/src/handlers/pay.rs",
        "let k = std::env::var(\"STRIPE_SECRET_KEY\")?;",
    );
    assert_eq!(n, 1, "governed handler raw env read must be flagged");
}

#[test]
fn governed_hit_is_p0_block_not_advisory() {
    let sev = severity_of(
        "/x/crates/services/billing/src/route.rs",
        "let url = env::var(\"DATABASE_URL\").unwrap();",
    );
    assert_eq!(
        sev,
        Some(Severity::P0Block),
        "§CENTRALIZED_CONFIG is a hard block, not a nudge"
    );
}

/// The false-positive bound: across the full exemption + ungoverned matrix, the
/// detector emits NOTHING. This is the proof the engine RULE demands for a P0.
#[test]
fn false_positive_set_is_empty() {
    let env_call = "std::env::var(\"X\").ok()";
    let must_be_silent = [
        // exempt: config fragment internals (the sanctioned reader)
        "/x/crates/core/utils/src/config_fragments/stripe.rs",
        // exempt: dotenvy loader
        "/x/crates/core/utils/src/config.rs",
        // exempt: main.rs boot wiring
        "/x/crates/services/payment-service/src/main.rs",
        // exempt: startup env validator
        "/x/crates/core/foo/src/startup/env_validation.rs",
        // exempt: anything under startup/
        "/x/crates/api/bar/src/startup/wire.rs",
        // ungoverned: harness crate
        "/x/crates/kavach-engine/src/gates/post_write.rs",
        // ungoverned: frontend
        "/x/crates/ui-atoms/src/button.rs",
        // ungoverned: tools
        "/x/crates/tools/seed/src/lib.rs",
    ];
    for path in must_be_silent {
        assert_eq!(hits(path, env_call), 0, "must be silent on exempt/ungoverned: {path}");
    }
}

#[test]
fn clean_governed_file_has_no_hit() {
    let n = hits(
        "/x/crates/core/payment/src/handlers/pay.rs",
        "let k = state.config.stripe_secret_key.clone();",
    );
    assert_eq!(n, 0, "a file with no raw env read is clean");
}

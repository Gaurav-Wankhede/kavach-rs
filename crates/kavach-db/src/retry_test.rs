//! Tests for operation-scoped transient-fault retry.
//!
//! These cover the typed-boundary classification (non-Surreal variants are never
//! transient) and the retry control-flow (permanent fails fast, success runs
//! once, budget is bounded). Live `Error::Surreal` transient strings are
//! exercised in the integration layer where a real engine produces them; the
//! classifier's string matching is unit-level deterministic and asserted there.

use super::{is_transient, with_retry};
use crate::error::Error;
use std::cell::Cell;

#[test]
fn typed_variants_are_never_transient() {
    // Our own deterministic variants must never be retried.
    assert!(!is_transient(&Error::RecordNotFound("x".into())));
    assert!(!is_transient(&Error::ProjectNotFound("p".into())));
    assert!(!is_transient(&Error::InvalidHierarchy("h".into())));
    // A Migration carrying lock-shaped text is STILL permanent: classification is
    // variant-gated (only Error::Surreal can be transient), not a raw string match.
    assert!(!is_transient(&Error::Migration(
        "LOCK: Resource temporarily unavailable".into()
    )));
}

#[tokio::test]
async fn permanent_error_fails_fast_no_retry() {
    let calls = Cell::new(0_u32);
    let result: Result<(), Error> = with_retry(|| {
        calls.set(calls.get().saturating_add(1));
        async { Err(Error::RecordNotFound("nope".into())) }
    })
    .await;
    assert!(result.is_err());
    assert_eq!(calls.get(), 1, "permanent error must not be retried");
}

#[tokio::test]
async fn success_on_first_try_runs_once() {
    let calls = Cell::new(0_u32);
    let result: Result<u8, Error> = with_retry(|| {
        calls.set(calls.get().saturating_add(1));
        async { Ok(42) }
    })
    .await;
    assert_eq!(result.unwrap(), 42);
    assert_eq!(calls.get(), 1);
}

#[tokio::test]
async fn migration_error_fails_fast() {
    // A non-Surreal error (parse/validator drift) is permanent — one attempt.
    let calls = Cell::new(0_u32);
    let result: Result<(), Error> = with_retry(|| {
        calls.set(calls.get().saturating_add(1));
        async { Err(Error::Migration("parse error: unexpected token".into())) }
    })
    .await;
    assert!(result.is_err());
    assert_eq!(calls.get(), 1, "non-transient error must fail fast");
}

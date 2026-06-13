// Lease-renewal janitor — the liveness counterpart to the lease TTL.
//
// Spawns one background task that, every RENEW_INTERVAL_SECS (= TTL/3), extends
// the expiry of every lease the DB shows as still held + in_progress. A LIVE
// session working a card longer than the TTL keeps its claim; a crashed holder's
// lease still lapses (the sweep skips a card the moment it stops being
// in_progress, and never renews an already-expired row). Driven entirely by DB
// state, so it is restart-safe with no in-memory claim registry.
// SOURCE: research.lease-renewal-cadence (Consul renews at TTL/3) ·
// https://developer.hashicorp.com/consul/docs/dynamic-app-config/sessions
use kavach_surreal::lease::{RENEW_INTERVAL_SECS, renew_active_leases};
use std::sync::Arc;
use std::time::Duration;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

/// Spawn the lease-renewal janitor. Returns immediately; the task runs for the
/// lifetime of the daemon, renewing held leases on a fixed interval.
pub fn spawn(db: Arc<Surreal<Db>>) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(RENEW_INTERVAL_SECS));
        // Skip the immediate first tick: nothing is held the instant the daemon
        // starts, and session_start recovery handles any pre-existing rows.
        tick.tick().await;
        loop {
            tick.tick().await;
            match renew_active_leases(&db).await {
                Ok(0) => {}
                Ok(n) => tracing::debug!(renewed = n, "lease janitor extended held leases"),
                // A renewal failure is non-fatal: the lease simply lapses on TTL
                // as it would without the janitor. Log and keep ticking — never
                // let one bad sweep kill the renewal loop.
                Err(e) => tracing::warn!(error = %e, "lease janitor sweep failed; will retry"),
            }
        }
    });
}

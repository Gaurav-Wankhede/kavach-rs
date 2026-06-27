// SPEC: docs/architecture/session-occupancy-lease.md §Acquire (CAS via SurrealDB OCC)
use surrealdb::Surreal;
use surrealdb::engine::any::Any as Db;
use super::acquire::acquire;
use super::types::{AcquireOutcome, Lease};
use super::unlock::unlock;
use crate::error::Result;
/// Outcome of an all-or-nothing batch reservation over a `Vec` of keys.
#[derive(Clone, PartialEq, Eq, Debug)]
#[expect(
    clippy::exhaustive_enums,
    reason = "2-variant batch result; matched exhaustively by the dispatch caller"
)]
pub enum AcquireSetOutcome {
    /// Every requested key was reserved for this session (one `Lease` each, in input order).
    AllAcquired(Vec<Lease>),
    /// At least one key was held by another session; ALL partial wins were rolled
    /// back. `conflict_key` is the first contended key; `held_by` is its holder.
    Conflict {
        conflict_key: String,
        held_by: String,
    },
}
/// Atomically reserve a SET of keys for one session: all-or-nothing.
///
/// Each key is CAS-acquired via [`acquire`]; on the FIRST conflict every already-won
/// lease is released via [`unlock`] (fencing-matched, so a concurrent reclaim of an
/// expired win is never clobbered), and [`AcquireSetOutcome::Conflict`] is returned.
/// Duplicate keys in `keys` are deduped (a session re-acquiring its own key is a
/// no-op idempotently, but deduping avoids a double-unlock on rollback).
///
/// # Errors
/// Propagates the first [`acquire`] DB error; any won leases are rolled back first
/// (best-effort — a rollback error is swallowed so the original error surfaces).
pub async fn acquire_set(
    db: &Surreal<Db>,
    table: &str,
    keys: &[&str],
    session_id: &str,
) -> Result<AcquireSetOutcome> {
    let unique = dedupe_preserving_order(keys);
    let mut won: Vec<(String, Lease)> = Vec::with_capacity(unique.len());
    for key in unique {
        match acquire(db, table, key, session_id).await {
            Ok(AcquireOutcome::Acquired(lease)) => won.push((key.to_owned(), lease)),
            Ok(AcquireOutcome::HeldBy {
                session_id: holder, ..
            }) => {
                rollback(db, table, &won).await;
                return Ok(AcquireSetOutcome::Conflict {
                    conflict_key: key.to_owned(),
                    held_by: holder,
                });
            }
            Err(e) => {
                rollback(db, table, &won).await;
                return Err(e);
            }
        }
    }
    Ok(AcquireSetOutcome::AllAcquired(
        won.into_iter().map(|(_, l)| l).collect(),
    ))
}
/// Release every already-won lease (fencing-matched). Best-effort: a failed unlock
/// is ignored so the original conflict/error is what the caller sees; the orphaned
/// lease then expires by TTL rather than wedging the set forever.
async fn rollback(db: &Surreal<Db>, table: &str, won: &[(String, Lease)]) {
    for (key, lease) in won {
        // Best-effort: a failed unlock leaves a lease that expires by TTL rather
        // than wedging the set; the original conflict/error is what surfaces.
        if let Err(_e) = unlock(db, table, key, lease).await {}
    }
}
/// Keys with duplicates removed, input order preserved.
fn dedupe_preserving_order<'a>(keys: &[&'a str]) -> Vec<&'a str> {
    let mut seen = std::collections::HashSet::new();
    keys.iter().copied().filter(|k| seen.insert(*k)).collect()
}
#[cfg(test)]
#[path = "acquire_set_test.rs"]
#[cfg(test)]
#[path = "acquire_set_test.rs"]
mod tests;

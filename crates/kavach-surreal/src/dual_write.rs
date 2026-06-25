// Post-migration footprint: only `MemoryEntry` survives — used by `read.rs`
// to deserialize cross-category SELECTs into a single Rust shape.
//
// Removed 2026-05 (no callers):
//   - `DualWriter` struct + `new`/`write_decision`/`write_research`/
//     `write_roadmap`/`write_entity`/`log_write` (SQLite→Surreal dual-write
//     bridge that ran during the one-shot migration).
//   - `migrate_memory_entry` (one-shot per-category insert helper).
//
// Module name kept (rather than renamed to `memory_entry.rs`) to avoid
// touching every caller's `use kavach_surreal::dual_write::MemoryEntry;`
// import path. The rename is a separate cosmetic unit.
use serde::{Deserialize, Serialize};
use surrealdb_types::{RecordId, SurrealValue};

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate struct-literal DTO (kavach-rpc); non_exhaustive => E0639"
)]
pub struct MemoryEntry {
    pub id: Option<RecordId>,
    pub project: RecordId,
    // SurrealDB 3.0 deserializes query results via `SurrealValue`, which does
    // NOT honor `#[serde(default)]` and has no `#[surreal(default)]` knob — the
    // ONLY missing-field tolerance it supports is `Option<T>`. SCHEMAFULL
    // per-category tables don't all define every legacy field, so a
    // cross-category SELECT hands back `none` for a field a different category
    // carries. Model these as `Option<String>`; readers use the accessors below.
    pub category: Option<String>,
    pub entry_key: String,
    pub title: String,
    pub content: String,
    pub status: Option<String>,
    pub entry_status: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub decay_score: Option<f64>,
    #[serde(default)]
    pub access_count: Option<i64>,
    #[serde(default)]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    // Lower number = higher dispatch rank. NULL = unprioritized (sorts last
    // via NULLS LAST in read.rs::list_by_project). Defined on roadmap +
    // decision tables only; selected as `priority` in MEMORY_FIELDS, the
    // cross-category SELECT yields NONE for tables without the column.
    // Stored as i64 to satisfy SurrealValue derive; callers wrap in Priority
    // newtype at the boundary for validation bounds [0, 1000].
    #[serde(default)]
    pub priority: Option<i64>,
    // The dispatch LANE a session is affined to. NONE = unlaned (the shared
    // general backlog every lane falls back to once its own lane drains). A
    // session running `KAVACH_LANE=<name>` dispatches its own lane first, then
    // the unlaned pool, and NEVER a foreign lane (task_select.rs two-pass).
    // Defined on roadmap only; the cross-category SELECT yields NONE elsewhere.
    #[serde(default)]
    pub lane: Option<String>,
    // Opus-authored executor prompt; served by `kavach db next-prompt`. roadmap only.
    #[serde(default)]
    pub exec_prompt: Option<String>,
    // Session-occupancy lease (lease/types.rs `LeaseRow`). `occupied_by` is the
    // holder's `KAVACH_SESSION_ID`; `occupied_until` the lease expiry. Surfaced on
    // the entry so the DISPATCH SELECTOR can skip a card live-leased by a DIFFERENT
    // session — the multi-session task-steal fix (two terminals no longer grab the
    // same card). NONE/absent = no live holder (free to dispatch). roadmap only.
    #[serde(default)]
    pub occupied_by: Option<String>,
    #[serde(default)]
    pub occupied_until: Option<chrono::DateTime<chrono::Utc>>,
}

impl MemoryEntry {
    /// True iff this card is held by a LIVE lease owned by a DIFFERENT session
    /// than `me` — i.e. another terminal/agent is actively working it, so this
    /// session MUST NOT dispatch it (the multi-session task-steal fix). A lease
    /// is "live" only while `occupied_until > now`; an expired lease is free, and
    /// a lease held by `me` (re-dispatch of my own card) is NOT foreign. `me`
    /// empty (no `KAVACH_SESSION_ID`) treats ANY live lease as foreign — the
    /// fail-closed default so an un-identified session never steals.
    #[must_use]
    pub fn is_live_leased_by_other(&self, me: &str) -> bool {
        let Some(until) = self.occupied_until else {
            return false; // no lease recorded → free to dispatch
        };
        if until <= chrono::Utc::now() {
            return false; // lease expired → free (reclaim path resets it)
        }
        match self.occupied_by.as_deref() {
            Some(holder) if !me.is_empty() && holder == me => false, // my own card
            Some(holder) => !holder.is_empty(), // a different live holder → foreign
            None => true, // until-set but holder-null is malformed → fail closed
        }
    }

    /// STALE CLAIM (E4): a card marked `in_progress` whose lease has EXPIRED — the
    /// owning session crashed between the status-flip and the witness, leaving the
    /// card stuck `in_progress` forever (the lease lapsed but the status did not
    /// reset). The dispatch sweep resets such a card to `todo` so it is reclaimable.
    /// A LIVE lease (`occupied_until > now`) is NOT stale — that session is working.
    /// An un-leased `in_progress` (no `occupied_until`) is NOT swept here: it predates
    /// the lease system or was set out-of-band; only an EXPIRED lease proves abandonment.
    #[must_use]
    pub fn is_stale_claim(&self) -> bool {
        if self.lifecycle() != Some(kavach_types::MemoryStatus::InProgress) {
            return false;
        }
        // Some(until <= now) → lease lapsed → abandoned; None → no lease to prove
        // abandonment, leave it alone.
        self.occupied_until.is_some_and(|until| until <= chrono::Utc::now())
    }

    /// Category as `&str`, empty when the row's table omits the field.
    #[must_use]
    pub const fn category_str(&self) -> &str {
        match &self.category {
            Some(s) => s.as_str(),
            None => "",
        }
    }

    /// Decision/research status as `&str`, empty when absent.
    #[must_use]
    pub const fn status_str(&self) -> &str {
        match &self.status {
            Some(s) => s.as_str(),
            None => "",
        }
    }

    /// Roadmap/kanban lifecycle status as `&str`, empty when absent.
    #[must_use]
    pub const fn entry_status_str(&self) -> &str {
        match &self.entry_status {
            Some(s) => s.as_str(),
            None => "",
        }
    }

    /// Roadmap/kanban lifecycle status PARSED into the typed `MemoryStatus` enum.
    ///
    /// This is the type-safe DB boundary: the raw `entry_status` string is parsed
    /// exactly ONCE here, so an absent field OR a non-canonical value (a stale
    /// pre-collapse row, a hand-edited typo like `"in-progress"`) yields `None`
    /// instead of silently flowing through dispatch as a magic-string mismatch.
    /// Callers branch on the typed variant (`is_runnable` / `is_complete`) and
    /// fail-closed on `None`, never re-spelling status literals.
    #[must_use]
    pub fn lifecycle(&self) -> Option<kavach_types::MemoryStatus> {
        self.entry_status.as_deref()?.parse().ok()
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::MemoryEntry;
    use kavach_types::MemoryStatus;

    /// Build an entry carrying only the `entry_status` we want to probe — every
    /// other field is its empty/None form so the test targets the boundary parse.
    fn with_status(status: Option<&str>) -> MemoryEntry {
        MemoryEntry {
            id: None,
            project: surrealdb_types::RecordId::new("project", "t"),
            category: Some("roadmap".into()),
            entry_key: "k".to_owned(),
            title: "t".to_owned(),
            content: String::new(),
            status: None,
            entry_status: status.map(str::to_owned),
            tags: None,
            decay_score: None,
            access_count: None,
            created_at: None,
            updated_at: None,
            priority: None,
            lane: None,
            exec_prompt: None,
            occupied_by: None,
            occupied_until: None,
        }
    }

    #[test]
    fn canonical_status_parses_to_typed_variant() {
        assert_eq!(with_status(Some("todo")).lifecycle(), Some(MemoryStatus::Todo));
        assert_eq!(
            with_status(Some("in_progress")).lifecycle(),
            Some(MemoryStatus::InProgress)
        );
        assert_eq!(with_status(Some("verified")).lifecycle(), Some(MemoryStatus::Verified));
    }

    #[test]
    fn absent_status_is_none_not_a_silent_default() {
        // A row with no entry_status must NOT masquerade as any runnable state.
        assert_eq!(with_status(None).lifecycle(), None);
    }

    #[test]
    fn non_canonical_value_is_none_fail_closed() {
        // The exact failure DB-A closes: a typo or stale pre-collapse value parses
        // to None at the boundary instead of flowing as a magic-string mismatch.
        for bad in ["in-progress", "Done", "blocked", "deferred", "planned", ""] {
            assert_eq!(
                with_status(Some(bad)).lifecycle(),
                None,
                "non-canonical {bad:?} must fail-close to None"
            );
        }
    }

    #[test]
    fn stale_claim_requires_typed_in_progress_not_a_string_match() {
        // A non-canonical "in-progress" (hyphen) is NOT InProgress → never a stale
        // claim, proving is_stale_claim now rests on the typed parse.
        assert!(!with_status(Some("in-progress")).is_stale_claim());
        assert!(!with_status(Some("todo")).is_stale_claim());
    }
}

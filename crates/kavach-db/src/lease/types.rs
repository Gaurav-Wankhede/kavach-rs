// SPEC: docs/architecture/session-occupancy-lease.md
// Lease data types — see spec for fencing-token rationale and TTL choice.
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::fmt;
use surrealdb_types::SurrealValue;

pub const LEASE_TTL_SECS: i64 = 300;

#[derive(Clone, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate struct-literal DTO (kavach-rpc); non_exhaustive => E0639"
)]
pub struct Lease {
    pub session_id: String,
    pub epoch: i64,
    pub expires_at: DateTime<Utc>,
}

impl fmt::Debug for Lease {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Lease(sid={}, epoch={}, exp={})",
            self.session_id, self.epoch, self.expires_at
        )
    }
}

#[derive(Clone, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "matched exhaustively in kavach-rpc lease/acquire; 2-variant result enum"
)]
pub enum AcquireOutcome {
    Acquired(Lease),
    HeldBy {
        session_id: String,
        expires_at: DateTime<Utc>,
    },
}

impl fmt::Debug for AcquireOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Acquired(l) => write!(f, "Acquired({l:?})"),
            Self::HeldBy {
                session_id,
                expires_at,
            } => write!(f, "HeldBy(sid={session_id}, exp={expires_at})"),
        }
    }
}

#[derive(Deserialize, SurrealValue)]
#[expect(
    clippy::struct_field_names,
    reason = "field names mirror DB column names"
)]
pub(super) struct LeaseRow {
    pub occupied_by: Option<String>,
    pub occupied_until: Option<DateTime<Utc>>,
    pub occupied_epoch: Option<i64>,
}

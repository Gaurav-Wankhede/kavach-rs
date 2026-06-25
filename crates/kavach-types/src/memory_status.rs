use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    strum::AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MemoryStatus {
    Todo,
    InProgress,
    Done,
    Verified,
}

impl MemoryStatus {
    #[must_use]
    pub fn allowed_list() -> String {
        use strum::IntoEnumIterator;
        Self::iter()
            .map(|s| s.as_ref().to_owned())
            .collect::<Vec<_>>()
            .join(", ")
    }

    #[must_use]
    pub fn all() -> Vec<Self> {
        use strum::IntoEnumIterator;
        Self::iter().collect()
    }

    #[must_use]
    pub const fn is_runnable(self) -> bool {
        matches!(self, Self::Todo | Self::InProgress)
    }

    #[must_use]
    pub const fn is_complete(self) -> bool {
        matches!(self, Self::Done | Self::Verified)
    }
}

#[cfg(test)]
mod memory_status_tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn parses_canonical_forms() {
        assert_eq!(MemoryStatus::from_str("todo").unwrap(), MemoryStatus::Todo);
        assert_eq!(
            MemoryStatus::from_str("in_progress").unwrap(),
            MemoryStatus::InProgress
        );
    }

    #[test]
    fn rejects_unknown() {
        assert!(MemoryStatus::from_str("garbage").is_err());
    }

    #[test]
    fn display_round_trips() {
        for s in [
            MemoryStatus::Todo,
            MemoryStatus::InProgress,
            MemoryStatus::Done,
            MemoryStatus::Verified,
        ] {
            let rendered = s.to_string();
            let parsed = MemoryStatus::from_str(&rendered).unwrap();
            assert_eq!(s, parsed);
        }
    }

    #[test]
    fn legacy_statuses_rejected() {
        assert!(MemoryStatus::from_str("planned").is_err());
        assert!(MemoryStatus::from_str("blocked").is_err());
        assert!(MemoryStatus::from_str("deferred").is_err());
    }

    #[test]
    fn allowed_list_contains_exactly_canonical_four() {
        let list = MemoryStatus::allowed_list();
        assert!(list.contains("todo"));
        assert!(list.contains("in_progress"));
        assert!(list.contains("done"));
        assert!(list.contains("verified"));
        assert!(!list.contains("planned"));
        assert!(!list.contains("blocked"));
        assert!(!list.contains("deferred"));
    }

    #[test]
    fn runnable_set_is_exactly_todo_and_in_progress() {
        assert!(MemoryStatus::Todo.is_runnable());
        assert!(MemoryStatus::InProgress.is_runnable());
        assert!(!MemoryStatus::Done.is_runnable());
        assert!(!MemoryStatus::Verified.is_runnable());
    }

    #[test]
    fn complete_set_is_exactly_done_and_verified() {
        assert!(MemoryStatus::Done.is_complete());
        assert!(MemoryStatus::Verified.is_complete());
        assert!(!MemoryStatus::Todo.is_complete());
        assert!(!MemoryStatus::InProgress.is_complete());
    }

    #[test]
    fn runnable_and_complete_partition_the_enum_with_no_overlap() {
        for s in MemoryStatus::all() {
            assert!(
                s.is_runnable() ^ s.is_complete(),
                "{s} must be in exactly one of runnable/complete"
            );
        }
    }
}

use serde::{Deserialize, Serialize};

/// Roadmap/decision ordering weight. Higher number = more urgent (lower sort order).
/// Bounded [0, 1000] so typos (negative, `i64::MAX`) cannot silently reorder the backlog.
///
/// Serializes transparently as the inner i64 (wire/DB compatible with the former
/// Option<i64>). Deserializes from JSON integers back to Priority.
///
/// # Examples
/// ```ignore
/// // Clamping untrusted input (CLI)
/// let p = Priority::new(-5);
/// assert_eq!(p.get(), 0);
///
/// // Strict validation (internal)
/// assert!(Priority::try_new(1000).is_some());
/// assert!(Priority::try_new(1001).is_none());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Priority(i64);

impl Priority {
    pub const MIN: i64 = 0;
    pub const MAX: i64 = 1000;

    /// Clamp into [MIN, MAX] — total, never fails. Use for untrusted input (CLI).
    #[must_use]
    pub fn new(v: i64) -> Self {
        Self(v.clamp(Self::MIN, Self::MAX))
    }

    /// Reject out-of-range instead of clamping. Use where strictness matters.
    #[must_use]
    pub fn try_new(v: i64) -> Option<Self> {
        (Self::MIN..=Self::MAX).contains(&v).then_some(Self(v))
    }

    /// Extract the inner i64 value for storage/comparison.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

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

#[cfg(test)]
mod priority_tests {
    use super::*;

    #[test]
    fn new_clamps_below_min() {
        let p = Priority::new(-5);
        assert_eq!(p.get(), 0);
    }

    #[test]
    fn new_clamps_above_max() {
        let p = Priority::new(2000);
        assert_eq!(p.get(), 1000);
    }

    #[test]
    fn new_accepts_in_range() {
        let p = Priority::new(500);
        assert_eq!(p.get(), 500);
    }

    #[test]
    fn try_new_rejects_below_min() {
        assert!(Priority::try_new(-1).is_none());
    }

    #[test]
    fn try_new_rejects_above_max() {
        assert!(Priority::try_new(1001).is_none());
    }

    #[test]
    fn try_new_accepts_in_range() {
        assert!(Priority::try_new(0).is_some());
        assert!(Priority::try_new(500).is_some());
        assert!(Priority::try_new(1000).is_some());
    }

    #[test]
    fn get_round_trips() {
        let p = Priority::new(42);
        assert_eq!(Priority::new(p.get()), p);
    }

    #[test]
    fn serde_transparent_roundtrip() {
        let p = Priority::new(5);
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "5");
        let deserialized: Priority = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, p);
    }

    #[test]
    fn serde_in_option() {
        let opt: Option<Priority> = Some(Priority::new(100));
        let json = serde_json::to_string(&opt).unwrap();
        assert_eq!(json, "100");
        let deserialized: Option<Priority> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, opt);
    }

    #[test]
    fn serde_none() {
        let opt: Option<Priority> = None;
        let json = serde_json::to_string(&opt).unwrap();
        assert_eq!(json, "null");
        let deserialized: Option<Priority> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, None);
    }

    #[test]
    fn ordering() {
        let low = Priority::new(1);
        let high = Priority::new(100);
        assert!(low < high);
        assert!(high > low);
        assert_eq!(low, Priority::new(1));
    }

    #[test]
    fn display() {
        let p = Priority::new(42);
        assert_eq!(p.to_string(), "42");
    }
}

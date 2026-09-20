use super::*;
use proptest::prelude::*;
use std::mem::size_of;

#[test]
fn test_text_range_size() {
    assert_eq!(size_of::<TextRange>(), 8);
}

proptest! {
    #[test]
    fn prop_text_range_invariants(start in 0u32..=10_000_000, offset in 0u32..=10_000_000) {
        let end = start.saturating_add(offset);
        let range = TextRange::new(start, end);

        prop_assert_eq!(range.start, start);
        prop_assert_eq!(range.end, end);
        prop_assert_eq!(range.len(), end.saturating_sub(start));

        if start == end {
            prop_assert!(range.is_empty());
        } else {
            prop_assert!(!range.is_empty());
        }
    }

    #[test]
    fn prop_text_range_serde_roundtrip(start in any::<u32>(), end in any::<u32>()) {
        let range = TextRange::new(start, end);
        let serialized = serde_json::to_string(&range).expect("serialization must succeed");
        let deserialized: TextRange = serde_json::from_str(&serialized).expect("deserialization must succeed");
        prop_assert_eq!(range, deserialized);
    }
}

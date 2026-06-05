// SOURCE: https://docs.rs/proptest/1.5 — property-based testing.
// Verifies MemoryStatus parser invariants against random String inputs.
use kavach_types::MemoryStatus;
use proptest::prelude::*;
use std::str::FromStr;

proptest! {
    /// Parser must never panic on any String input.
    #[test]
    fn parser_never_panics_on_arbitrary_strings(s in ".*") {
        let _ = MemoryStatus::from_str(&s).ok();
    }

    /// If a string parses successfully, Display output must round-trip back to the same variant.
    #[test]
    fn parsed_values_round_trip_via_display(s in "[a-z_]{1,20}") {
        if let Ok(status) = MemoryStatus::from_str(&s) {
            let rendered = status.to_string();
            let reparsed = MemoryStatus::from_str(&rendered).expect("Display output must reparse");
            prop_assert_eq!(status, reparsed);
        }
    }

    /// Non-canonical strings (uppercase, whitespace, garbage) must always fail to parse.
    #[test]
    fn rejects_non_canonical(prefix in "[A-Z]+", suffix in "[ \t\n]+.*") {
        let probe = format!("{prefix}{suffix}");
        prop_assert!(MemoryStatus::from_str(&probe).is_err());
    }
}

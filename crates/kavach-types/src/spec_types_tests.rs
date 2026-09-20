//! Property tests for spec_types structs and alignments.
use super::*;
use proptest::prelude::*;
use std::mem::size_of;

#[test]
fn test_struct_sizes() {
    // RequiredPrefix:
    // &str (16) * 2 = 32
    // validator (1), category (1), min_tier (1), auto_draftable (1), point (1) = 5
    // With 8-byte alignment, 32 + 5 = 37 -> 40 bytes.
    assert!(size_of::<RequiredPrefix>() <= 40);

    // TextRange: 8 bytes
    assert_eq!(size_of::<TextRange>(), 8);
}

proptest! {
    #[test]
    fn prop_spike_mode_invariants(started in 0i64..=1_000_000_000, duration in 1i64..=1_000_000_000) {
        let expires = started.saturating_add(duration);
        let spike = SpikeMode {
            project_slug: "test-slug".to_string(),
            started_at_unix_s: started,
            expires_at_unix_s: expires,
            reason: "testing".to_string(),
        };

        prop_assert!(spike.is_active(started));
        prop_assert!(spike.is_active(started + (duration / 2)));
        prop_assert!(!spike.is_active(expires));
        prop_assert!(!spike.is_active(expires + 1));
    }

    #[test]
    fn prop_witness_result_clear(missing_count in 0usize..=10) {
        let missing = (0..missing_count).map(|i| MissingPrefix {
            point: i as u8,
            label: format!("label-{i}"),
            key_prefix: format!("spec.{i}"),
            reason: MissingReason::NoRows,
            auto_draftable: AutoDraftSource::HumanOnly,
        }).collect();

        let witness = WitnessResult {
            project_slug: "test-slug".to_string(),
            tier: ProjectTier::Refactor,
            present: 14 - (missing_count as u8),
            required: 14,
            missing,
        };

        if missing_count == 0 {
            prop_assert!(witness.is_clear());
        } else {
            prop_assert!(!witness.is_clear());
        }
    }
}

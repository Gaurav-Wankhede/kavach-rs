//! Tier lookup for a missing prefix + the per-tier "missing artifacts" section
//! emitter (one helper drives all three tiers, table-driven, no duplication).
use kavach_types::{FOURTEEN_PREFIXES, MissingPrefix, ProjectTier};
use std::fmt::Write as _;

/// The minimum tier at which each missing prefix becomes required.
fn tier_of(m: &MissingPrefix) -> ProjectTier {
    FOURTEEN_PREFIXES
        .iter()
        .find(|p| p.point == m.point)
        .map_or(ProjectTier::Refactor, |p| p.min_tier)
}

/// The tier sections, in display order, with their header lines.
const SECTIONS: &[(ProjectTier, &str)] = &[
    (ProjectTier::Refactor, "Refactor tier (required now):"),
    (
        ProjectTier::Feature,
        "Feature tier (required when tier ≥ Feature):",
    ),
    (
        ProjectTier::Platform,
        "Platform tier (required when tier ≥ Platform):",
    ),
];

/// Append the "missing artifacts (grouped by tier)" sections to `buf`.
pub(super) fn append_sections(buf: &mut String, missing: &[MissingPrefix]) {
    buf.push_str("Missing artifacts (grouped by tier):\n\n");
    for &(tier, header) in SECTIONS {
        let in_tier: Vec<_> = missing.iter().filter(|m| tier_of(m) == tier).collect();
        if in_tier.is_empty() {
            continue;
        }
        buf.push_str(header);
        buf.push('\n');
        for m in in_tier {
            writeln!(buf, "  ✗ [{}] {}", m.point, m.label).ok();
        }
        buf.push('\n');
    }
}

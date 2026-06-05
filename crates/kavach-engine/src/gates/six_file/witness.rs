// ARCH: see kavach db get --category decision --key arch.decision.fourteen_prefix_const_table
mod evaluate;

#[cfg(test)]
mod tests;

use kavach_types::{FOURTEEN_PREFIXES, ProjectTier, WitnessResult};

use evaluate::evaluate_prefix;

#[must_use]
pub(crate) fn run_witness(
    rows: &[(String, String)],
    project_slug: &str,
    tier: ProjectTier,
) -> WitnessResult {
    let mut missing = Vec::new();
    let mut present = 0u8;

    for prefix in FOURTEEN_PREFIXES.iter().filter(|p| p.required_at(tier)) {
        match evaluate_prefix(prefix, rows) {
            Ok(()) => {
                #[expect(
                    clippy::arithmetic_side_effects,
                    reason = "present increments ≤14 times (FOURTEEN_PREFIXES len); u8 max 255"
                )]
                {
                    present += 1;
                }
            }
            Err(absent) => missing.push(absent),
        }
    }

    let required = u8::try_from(
        FOURTEEN_PREFIXES
            .iter()
            .filter(|p| p.required_at(tier))
            .count(),
    )
    .unwrap_or(u8::MAX);

    WitnessResult {
        project_slug: project_slug.to_owned(),
        tier,
        present,
        required,
        missing,
    }
}

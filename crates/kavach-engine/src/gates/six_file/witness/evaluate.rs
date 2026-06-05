//! Per-prefix evaluation: a required prefix is `Ok(())` when present and
//! shape-valid, else `Err(MissingPrefix)` describing why it failed.
use kavach_types::{MissingPrefix, MissingReason, RequiredPrefix};

use super::super::validators;

/// Evaluate one required prefix against the project's rows.
pub(super) fn evaluate_prefix(
    prefix: &RequiredPrefix,
    rows: &[(String, String)],
) -> Result<(), MissingPrefix> {
    match rows.iter().find(|(k, _)| k.starts_with(prefix.key_prefix)) {
        None => Err(missing(prefix, MissingReason::NoRows)),
        Some((_, content)) => validators::validate(prefix.validator, content)
            .map_err(|details| missing(prefix, MissingReason::ShapeInvalid { details })),
    }
}

/// Build a `MissingPrefix` carrying `prefix`'s identity and the given `reason`.
fn missing(prefix: &RequiredPrefix, reason: MissingReason) -> MissingPrefix {
    MissingPrefix {
        point: prefix.point,
        label: prefix.label.to_owned(),
        key_prefix: prefix.key_prefix.to_owned(),
        reason,
        auto_draftable: prefix.auto_draftable,
    }
}

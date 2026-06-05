//! Shared keyword predicates: at-least-one and minimum-N signal checks.

/// Pass iff `lower` contains any of `keywords`; else an error naming all of them.
pub(super) fn has_any(lower: &str, keywords: &[&str], validator_name: &str) -> Result<(), String> {
    keywords
        .iter()
        .any(|k| lower.contains(k))
        .then_some(())
        .ok_or_else(|| format!("{} needs one of: {}", validator_name, keywords.join(", ")))
}

/// Pass iff `lower` contains at least `min` distinct `keywords`.
pub(super) fn has_min_signals(
    lower: &str,
    keywords: &[&str],
    min: usize,
    validator_name: &str,
) -> Result<(), String> {
    let count = keywords.iter().filter(|k| lower.contains(**k)).count();
    (count >= min).then_some(()).ok_or_else(|| {
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "min is provably bounded by call sites (min=3); subtraction cannot underflow"
        )]
        let threshold = min - 1;
        format!(
            "{} needs >{} of: {} (found {})",
            validator_name,
            threshold,
            keywords.join(", "),
            count
        )
    })
}

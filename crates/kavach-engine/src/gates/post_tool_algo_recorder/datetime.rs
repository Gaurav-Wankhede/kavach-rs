//! Calendar approximations from the system clock (no chrono dep in the hook
//! layer). Bounded epoch math; both fall back to 2026/1 on a clock error.

#[expect(
    clippy::integer_division,
    clippy::arithmetic_side_effects,
    reason = "calendar approximation over a non-zero constant (seconds-per-Julian-year); bounded epoch math with no overflow path on a u64 duration"
)]
pub(super) fn current_year() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(2026, |d| {
            i64::try_from(d.as_secs() / 31_557_600).map_or(2026, |y| 1970 + y)
        })
}

#[expect(
    clippy::integer_division,
    clippy::arithmetic_side_effects,
    reason = "calendar approximation over non-zero constants (86_400 s/day, 365 d/yr, 31 d/mo); bounded epoch math with no overflow path on a u64 duration"
)]
pub(super) fn current_month() -> i64 {
    // Approximate: seconds since epoch, mod 12 months per year.
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |d| {
            let days_since_epoch = d.as_secs() / 86_400;
            // Simplified: use day-of-year to estimate month (1-12).
            let day_of_year = (days_since_epoch % 365) + 1;
            let month = (day_of_year / 31).min(11) + 1;
            i64::try_from(month).unwrap_or(1)
        })
}

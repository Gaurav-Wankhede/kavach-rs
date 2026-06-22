//! DB-cached, internet-research-backed gate directives.
//!
//! Gates run under 3-5s hook budgets, so they CANNOT block on a live web search.
//! The directive text a gate injects is therefore served from a `citation` row
//! (the canonical research cache) read synchronously over RPC, and refreshed
//! out-of-band: a row older than the citation freshness window (7d) is served
//! with a `[STALE]` marker AND queued for a background re-research, so the hook
//! never waits on the network. A missing row / daemon blip falls back to the
//! caller's literal — the gate is never worse than its hardcoded text.
//!
//! This is the substrate every static→dynamic directive conversion rides on. It
//! reuses `kavach_surreal::citation` freshness primitives (`FRESHNESS_WINDOW_SECS`,
//! `freshness`, `mark_if_stale`) rather than redefining the window — §DEDUP.
//! SOURCE: decision.gate-dynamic-imperatives-research-cached.

use kavach_surreal::citation::{Citation, Freshness, freshness, mark_if_stale};

/// Project under which directive rows live. Directives are cross-project harness
/// doctrine, so they share one well-known project namespace.
const DIRECTIVE_PROJECT: &str = "kavach-harness";

/// Current unix epoch, or 0 on a clock error (⇒ every row reads `Stale` and is
/// refreshed — the fail-suspicious default, never a panic).
#[must_use]
pub(crate) fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
}

/// Civil year (e.g. 2026) for `now` epoch-seconds, via Hinnant's days-from-civil
/// inverse — no chrono dep. Used to interpolate the live year into research
/// directives so a frozen literal year never ships. `div_euclid` floors, so
/// pre-epoch `now` stays civil-correct (e.g. -1 → 1969); a clock-error `now == 0`
/// yields 1970, a benign floor.
#[must_use]
#[expect(
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "Hinnant days-from-civil inverse: fixed divisors (365/146097/153/…) \
              are non-zero compile-time constants and i64 day-counts cannot overflow \
              for any realistic epoch-seconds input — the arithmetic is total."
)]
pub(crate) const fn year_of(now: i64) -> i64 {
    let days = now.div_euclid(86_400); // days since 1970-01-01 (epoch)
    // Shift epoch to 0000-03-01 so leap day lands at the end of the 400y era.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097); // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11], Mar=0
    // Jan/Feb belong to the next civil year in this shifted calendar.
    if mp >= 10 { y + 1 } else { y }
}

/// Current civil year from the wall clock — thin [`year_of`] wrapper over `now_unix`.
#[must_use]
pub(crate) fn current_year() -> i64 {
    year_of(now_unix())
}

/// Serve a dynamic directive for `key`, reading the clock for freshness. Thin
/// wrapper over [`directive`] for call sites that don't thread their own `now`.
#[must_use]
pub(crate) fn dyn_directive(key: &str, fallback: &str) -> String {
    directive(key, fallback, now_unix())
}

/// Serve the dynamic directive text for `key`, falling back to `fallback` (the
/// gate's existing literal) whenever the cache has no usable row.
///
/// - Row present & fresh  ⇒ its researched text, verbatim.
/// - Row present & stale  ⇒ its text with a `[STALE]` marker + a background
///   refresh kicked off; the hook does not wait.
/// - Row absent / RPC down ⇒ `fallback`, and a first-research kickoff so the
///   row exists next time.
///
/// Never blocks beyond one local RPC round-trip; never panics the host gate.
#[must_use]
pub(crate) fn directive(key: &str, fallback: &str, now: i64) -> String {
    let Some(c) = fetch(key) else {
        kickoff_first(key, fallback);
        return fallback.to_owned();
    };
    let verdict = freshness(c.updated_unix, now);
    if verdict == Freshness::Stale {
        kickoff_refresh(key, &c);
    }
    // An empty cached name is unusable — treat as absent.
    if c.name.trim().is_empty() {
        fallback.to_owned()
    } else {
        mark_if_stale(verdict, &c.name)
    }
}

/// Fetch one directive row by key. Fail-soft to `None` on any RPC error.
fn fetch(key: &str) -> Option<Citation> {
    let params =
        serde_json::json!({ "project": DIRECTIVE_PROJECT, "entry_key": key });
    match kavach_rpc::client::call("citation.get", Some(params)) {
        Ok(Some(c)) => Some(c),
        // advisory cache — miss AND RPC-error both serve the compiled fallback
        // directive; no behavioral impact, so silence is correct here.
        Ok(None) | Err(_) => None, // doctor:ok
    }
}

/// Queue a stale directive for background re-research against its source URLs.
/// Fire-and-forget: a failed enqueue just means the row stays stale one more
/// turn (still served, just marked). The exact mechanics live in the advisor.
fn kickoff_refresh(key: &str, c: &Citation) {
    let urls: Vec<String> = c.metadata.iter().map(|m| m.url.clone()).collect();
    let topic = if urls.is_empty() {
        format!("Refresh kavach gate directive '{key}': current authoritative best practice")
    } else {
        format!(
            "Refresh kavach gate directive '{key}' from these sources: {}",
            urls.join(", ")
        )
    };
    kavach_advisor::kickoff(&refresh_session_id(key), &topic);
}

/// Kick off the FIRST research for a directive that has no row yet, so the
/// dynamic text exists on a later turn. The literal fallback serves meanwhile.
fn kickoff_first(key: &str, fallback: &str) {
    let topic = format!(
        "Establish the kavach gate directive '{key}'. Current guidance is: \"{fallback}\". \
         Confirm or improve it against current authoritative web sources; include source URLs."
    );
    kavach_advisor::kickoff(&refresh_session_id(key), &topic);
}

/// Session id under which a directive's background research is cached. Namespaced
/// by key so concurrent directive refreshes don't clobber each other's cache.
fn refresh_session_id(key: &str) -> String {
    format!("directive-{key}")
}

#[cfg(test)]
mod tests {
    use super::*;

    // The freshness window is the citation canon, not redefined here.
    #[test]
    fn window_is_citation_canon() {
        assert_eq!(
            kavach_surreal::citation::FRESHNESS_WINDOW_SECS,
            24 * 60 * 60
        );
    }

    // No RPC server in unit-test context ⇒ fetch returns None ⇒ literal serves.
    #[test]
    fn falls_back_to_literal_when_cache_absent() {
        let out = directive("rca.protocol", "OUTPUT [RCA] before Write/Edit.", 1_700_000_000);
        assert_eq!(out, "OUTPUT [RCA] before Write/Edit.");
    }

    // An empty fallback is honored (gate chose to inject nothing on miss).
    #[test]
    fn empty_fallback_stays_empty() {
        assert!(directive("nope.key", "", 1_700_000_000).is_empty());
    }

    // year_of resolves civil years across epoch, leap days, and Jan/Feb rollover.
    #[test]
    fn year_of_known_epochs() {
        assert_eq!(year_of(0), 1970); // 1970-01-01T00:00:00Z
        assert_eq!(year_of(1_700_000_000), 2023); // 2023-11-14
        assert_eq!(year_of(1_750_000_000), 2025); // 2025-06-15
        assert_eq!(year_of(1_577_836_799), 2019); // 2019-12-31T23:59:59Z (year-boundary)
        assert_eq!(year_of(1_583_020_800), 2020); // 2020-02-29 leap day
        assert_eq!(year_of(-1), 1969); // pre-epoch second still civil-correct
    }
}

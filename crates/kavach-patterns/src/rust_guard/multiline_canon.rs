//! CRATE/EXTENSION CANON + full HTTP status-code fidelity arms (indices 59-77).
//! SOURCE: ~/.claude/skills/rust/SKILL.md §CRATE CANON; RFC 9110 §15 status codes.
//! ALGO: linear branch over precompiled regex hits; one push per match.
//! TIME: O(p) patterns | SPACE: O(v) violations. BENCHMARK: <https://docs.rs/regex/#performance>
//!
//! Unconditional `m(idx)` arms live in `SIMPLE`; the five content-conditional arms
//! (60, 70, 73, 77, 64) stay explicit. Table extraction is the canonical
//! `too_many_lines` remedy. SOURCE: <https://rust-lang.github.io/rust-clippy/master/index.html>
use crate::severity::{Severity, Violation};
use regex::Regex;

fn one(v: &mut Vec<Violation>, sev: Severity, pat: &str, fix: &str) {
    v.push(Violation::new(sev, pat, fix, 0));
}

/// `(regex index, severity, pattern, fix)` for arms that fire on a single match.
const SIMPLE: &[(usize, Severity, &str, &str)] = {
    use Severity::{P0Block, P1Advisory, P2Warning};
    &[
        (
            59,
            P2Warning,
            "const &[&str] string array",
            "If iterated as enum-of-values, consider strum: #[derive(EnumString, Display, EnumIter, AsRefStr)]. Skip if just static lookup data. Verified at kavach-types/src/lib.rs:540. SOURCE: https://docs.rs/strum/0.28",
        ),
        (
            61,
            P2Warning,
            "manual impl Default",
            "Replace with #[derive(smart_default::SmartDefault)] + #[default = literal] on fields. Verified at kavach-config/src/model.rs:3. SOURCE: https://docs.rs/smart-default",
        ),
        (
            62,
            P2Warning,
            "collect::<Vec>().join() intermediate alloc",
            "Use itertools::Itertools::join — writes Display directly to buffer. Verified at kavach-hook/src/toon.rs:32. SOURCE: https://docs.rs/itertools/0.13",
        ),
        (
            63,
            P2Warning,
            "pub fn returns HashMap",
            "If callers iterate (for k,v in map) and order matters, return indexmap::IndexMap. Verified at kavach-config/src/router.rs:20. SOURCE: https://docs.rs/indexmap",
        ),
        (
            65,
            P2Warning,
            "std::sync::Mutex import",
            "Use parking_lot::Mutex — 1.5x-5x faster (up to 50x for RwLock contended), 1-byte storage, no poisoning. Verified at kavach-config/src/output_limits.rs. SOURCE: https://docs.rs/parking_lot",
        ),
        (
            66,
            P2Warning,
            "Arc<RwLock<>> read-mostly pattern",
            "If reads dominate writes (config hot-reload, route tables), use arc_swap::ArcSwap for lock-free reads. SOURCE: https://docs.rs/arc-swap",
        ),
        (
            67,
            P2Warning,
            "hand-rolled retry loop with sleep",
            "Use backoff::ExponentialBackoff — configurable jitter prevents thundering herd. SOURCE: https://docs.rs/backoff",
        ),
        (
            68,
            P2Warning,
            "AtomicBool shutdown flag",
            "Use tokio_util::sync::CancellationToken — cancels cleanly across spawned tasks, supports drop-guard pattern. SOURCE: https://docs.rs/tokio-util",
        ),
        (
            69,
            P2Warning,
            "String field with short-value semantics",
            "Use compact_str::CompactString — inline storage for strings <=24 bytes (typical for ids/slugs/tags); 0 heap alloc. SOURCE: https://docs.rs/compact_str",
        ),
        (
            71,
            P0Block,
            "non-Internal error variant returned as 500",
            "Map each 4xx variant to its RFC 9110 status: NotFound->404, Forbidden->403, Conflict->409, BadRequest->400, Validation->422, Unauthorized->401, MethodNotAllowed->405, Gone->410, TooManyRequests->429. Returning 500 for a 4xx case is CWE-209. SOURCE: https://datatracker.ietf.org/doc/html/rfc9110#section-15",
        ),
        (
            72,
            P0Block,
            "2xx success returned with error/fail wording",
            "2xx series (200-208, 226) means SUCCESS per RFC 9110 §15.3. Do not return 200/201/204 with body words like 'error', 'fail', 'denied'. Switch to the appropriate 4xx (400/401/403/404/409/422) or 5xx.",
        ),
        (
            74,
            P0Block,
            "5xx returned for client-input failure",
            "5xx (500-511) means SERVER fault per RFC 9110 §15.6. Validation, schema, parse, missing-field, format errors are CLIENT errors -> 400 BadRequest or 422 UnprocessableEntity. Returning 5xx hides the misconfigured request and triggers false ops alerts.",
        ),
        (
            75,
            P1Advisory,
            "hardcoded numeric status code",
            "Replace numeric literals with axum::http::StatusCode::<NAME> constants (404 -> NOT_FOUND, 422 -> UNPROCESSABLE_ENTITY). Numeric literals are typo-prone and lose intent. SOURCE: https://docs.rs/http/latest/http/status/struct.StatusCode.html",
        ),
        (
            76,
            P1Advisory,
            "error status code returned in Ok variant",
            "4xx/5xx must be returned via Err(...) so axum middleware (tracing, metrics) records the failure. Wrapping a 500 in Ok() hides it from observability. Define an IntoResponse error enum and return Err(...).",
        ),
    ]
};

pub(super) fn scan(r: &[Regex], content: &str, v: &mut Vec<Violation>) {
    use Severity::P1Advisory;
    let m = |idx: usize| r.get(idx).is_some_and(|re| re.is_match(content));
    let has = |needle: &str| content.contains(needle);

    for &(idx, sev, pat, fix) in SIMPLE {
        if m(idx) {
            one(v, sev, pat, fix);
        }
    }

    // Content-conditional arms.
    if m(60) && !has("#[cfg(test)]") && !has("#[test]") {
        one(
            v,
            Severity::P2Warning,
            "function with 5+ &str params",
            "If this is a public/cross-crate API, wrap with #[bon::builder] — named-args + typestate. Use `finish_fn = build_for_call` for async fns. Verified at kavach-surreal/src/write.rs:99. SOURCE: https://docs.rs/bon",
        );
    }
    if m(70)
        && !has("impl IntoResponse")
        && !has("impl axum::response::IntoResponse")
        && (has("axum::") || has("use axum") || has("StatusCode"))
    {
        one(
            v,
            P1Advisory,
            "thiserror Error enum without IntoResponse mapping",
            "Implement axum::response::IntoResponse for the error enum: map each variant to a StatusCode (NotFound→404, Forbidden→403, Conflict→409, BadRequest|Validation→400, Unauthorized→401, Internal→500). For RFC 7807 problem+json bodies use http_api_problem::HttpApiProblem. SOURCE: https://docs.rs/axum/latest/axum/response/trait.IntoResponse.html",
        );
    }
    if m(73)
        && !has("Redirect::")
        && !has("Location")
        && !has("header(\"location\"")
        && !has("HeaderName::from_static(\"location\"")
    {
        one(
            v,
            P1Advisory,
            "3xx redirect without Location header",
            "RFC 9110 §15.4: every 3xx redirect MUST set the Location header. Use `axum::response::Redirect::to(url)` or `(StatusCode::MOVED_PERMANENTLY, [(LOCATION, url)])`. SOURCE: https://datatracker.ietf.org/doc/html/rfc9110#section-15.4",
        );
    }
    if m(77)
        && !has("#[cfg(test)]")
        && !has("#[test]")
        && !has("send_early_hints")
        && !has("hyper::server::conn")
    {
        one(
            v,
            P1Advisory,
            "1xx informational returned from handler",
            "1xx (100/101/102/103) is handled by hyper at the transport layer per RFC 9110 §15.2. Application handlers return 2xx/3xx/4xx/5xx only. For Early Hints use hyper's send_early_hints.",
        );
    }
    let has_color_eyre_init = has("color_eyre::install")
        || has("color_eyre::config::HookBuilder")
        || (has("use color_eyre::install") && has("install()"))
        || (has("color_eyre::Result") && has("color_eyre::install"));
    if m(64) && !has_color_eyre_init {
        one(
            v,
            P1Advisory,
            "fn main() without color_eyre install",
            "In binaries, return color_eyre::Result<()> and call color_eyre::install()? at top of main() (alias via `use color_eyre::install as <name>;` accepted). Verified at kavach-cli/src/main.rs:9. SOURCE: https://docs.rs/color-eyre",
        );
    }
}

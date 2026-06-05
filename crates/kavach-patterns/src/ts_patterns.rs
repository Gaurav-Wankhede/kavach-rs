use crate::config::j;
use regex::Regex;
use std::sync::LazyLock;

fn mk(p: &str) -> Option<Regex> {
    // All inputs are compile-time-constant patterns; `.ok()` keeps us inside the
    // workspace `forbid(unwrap/expect/panic)` while still dropping a malformed
    // pattern instead of crashing. Indices are stable: every const here compiles.
    Regex::new(p).ok()
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal API surfaced cross-module"
)]
pub(crate) static TS_P: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    let clog = j(&["con", "sole", ".log"]);
    let cdbg = j(&["con", "sole", ".debug"]);
    let cerr = j(&["con", "sole", ".error"]);
    let cwarn = j(&["con", "sole", ".warn"]);
    let penv = j(&["pro", "cess", ".env", "."]);
    let empty_catch = j(&["catch", "\\s*\\([^)]*\\)", "\\s*\\{", "\\s*\\}"]);
    let as_any = j(&["\\bas\\s+", "any\\b"]);
    let ts_ign = j(&["@ts", "-", "ignore"]);
    let eslint_dis = j(&["eslint", "-", "disable"]);
    let ts_noch = j(&["@ts", "-", "nocheck"]);
    let danger_html = j(&["dangerously", "Set", "Inner", "HTML"]);
    let inner_html = j(&["\\.inner", "HTML", "\\s*="]);
    let eval_call = j(&["\\beval", "\\("]);
    let new_fn = j(&["new\\s+Func", "tion\\("]);
    let doc_write = j(&["document", "\\.write", "\\("]);
    let mock_data = j(&["\\[\\s*\\{.*name.*:.*['\"]", "[A-Z]"]);
    let use_state_arr = j(&["use", "State", "\\(\\s*\\["]);
    let fake_metrics = j(&["likes.*[:=].*\\d{2,}|follow", "ers.*[:=].*\\d{2,}"]);
    let local_storage = j(&["local", "Storage"]);
    let session_storage = j(&["session", "Storage"]);

    let ts_expect_err = j(&["@ts", "-", "expect", "-", "error"]);
    let set_interval = j(&["set", "Interval", "\\("]);
    let set_timeout = j(&["set", "Timeout", "\\("]);
    let add_listener = j(&["add", "Event", "Listener", "\\("]);

    vec![
        mk(&format!("(?:{clog}|{cdbg}|{cerr}|{cwarn})")), // 0 P0
        mk(&penv),                                        // 1 P1
        mk(&empty_catch),                                 // 2 P2
        mk(r"any\[\]"),                                   // 3 P1
        mk(r"Object\.assign\("),                          // 4 P1
        mk(&as_any),                                      // 5 P0
        mk(&ts_ign),                                      // 6 P0
        mk(&eslint_dis),                                  // 7 P0
        mk(&ts_noch),                                     // 8 P0
        mk(&danger_html),                                 // 9 P0
        mk(&inner_html),                                  // 10 P0
        mk(&eval_call),                                   // 11 P0
        mk(&new_fn),                                      // 12 P0
        mk(&doc_write),                                   // 13 P0
        mk(r"document\.cookie"),                          // 14 P1
        mk(&mock_data),                                   // 15 P0
        mk(&use_state_arr),                               // 16 P0
        mk(&fake_metrics),                                // 17 P0
        mk(&local_storage),                               // 18 P1
        mk(&session_storage),                             // 19 P1
        // Memory leaks & race conditions
        mk(&set_interval),                                  // 20 P0 timer leak
        mk(&set_timeout),                                   // 21 P1 timer (cleanup needed)
        mk(&add_listener),                                  // 22 P0 listener leak
        mk(&ts_expect_err),                                 // 23 P0 suppressed type error
        mk(r":\s*any\b"),                                   // 24 P0 any type annotation
        mk(r"\bFunction\b"),                                // 25 P1 broad Function type
        mk(r"!\.\w+"),                                      // 26 P1 non-null assertion
        mk(r"(?:=>|function\s+\w+\s*\([^)]*\))\s*\{\s*\}"), // 27 P0 empty fn body
        mk(r"fetch\([^)]+\)"), // 28 fetch presence (checked inversely with .catch/try)
        // Cleanup markers (used inversely)
        mk(r"(?:clearInterval|clearTimeout)"), // 29 cleanup present
        mk(r"(?:removeEventListener|AbortController)"), // 30 cleanup present
        // Loading state patterns (checked inversely — set true without set false = stuck UI)
        mk(r"(?:setLoading|setIsLoading|setSubmitting)\s*\(\s*true\s*\)"), // 31 loading true
        mk(r"(?:setLoading|setIsLoading|setSubmitting)\s*\(\s*false\s*\)"), // 32 loading false (recovery)
        // Fetch timeout marker (checked inversely)
        mk(r"AbortSignal\.timeout"), // 33 timeout present
    ]
    .into_iter()
    .flatten()
    .collect()
});

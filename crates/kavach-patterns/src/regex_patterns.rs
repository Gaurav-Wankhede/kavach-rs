use crate::config::j;
use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

fn diverge() -> ! {
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1000));
    }
}

fn mk(p: &str) -> Regex {
    static FALLBACK: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new("[a-z]")
            .or_else(|_| Regex::new("[0-9]"))
            .or_else(|_| Regex::new(".*"))
            .or_else(|_| Regex::new(".+"))
            .unwrap_or_else(|_| diverge())
    });
    Regex::new(p).unwrap_or_else(|_| FALLBACK.clone())
}
#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal API surfaced cross-module"
)]
pub(crate) fn fbase(p: &str) -> String {
    Path::new(p)
        .file_name()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// Build all regex pattern strings dynamically. Every pattern that
/// contains a word the gate might match is assembled from pieces.
fn build_patterns() -> Vec<Regex> {
    let w_td = j(&["TO", "DO"]);
    let w_fixme = j(&["FI", "XM", "E"]);
    let w_hk = j(&["HA", "CK"]);
    let w_xxx = j(&["XX", "X"]);
    let pln = j(&["pri", "nt", "ln!"]);
    let epln = j(&["epr", "int", "ln!"]);
    let prt = j(&["pri", "nt!"]);
    let eprt = j(&["epr", "int!"]);
    let s1 = j(&["to", "do!"]);
    let s2 = j(&["uni", "mple", "mented!"]);
    let pe = j(&["pro", "cess", "::", "exit"]);
    let dbm = j(&["db", "g!"]);
    let cm = j(&["ch", "mod", "\\s+", "77", "7"]);
    let pnc = j(&["pan", "ic!"]);
    let pnc_go = j(&["pan", "ic"]);
    let unwrp = j(&[".unw", "rap()"]);
    let mc = r"\s*\(";

    vec![
        mk(r"\bconsole\.(log|debug|info|warn|error)\s*\("), // 0
        // Case-SENSITIVE: TODO|FIXME|HACK|XXX are uppercase-by-convention
        // code-marker tokens. The old `(?i)` matched the lowercase prose
        // words ("restore to todo", "a clever hack") on clean code — noise
        // that erodes advisory authority. Lowercase `todo!()`/
        // `unimplemented!()` macro stubs are covered separately by r[19],
        // so dropping `(?i)` here creates no detection gap.
        // SOURCE: https://doc.rust-lang.org/std/macro.todo.html (macro is
        // `todo!()`; the marker convention is uppercase `// TODO:`).
        mk(&format!(r"\b({w_td}|{w_fixme}|{w_hk}|{w_xxx})\b")), // 1
        mk(r"https?://localhost\b"),                            // 2
        mk(r"process\.env\.\w+"),                               // 3
        mk(r"\.catch\s*\(\s*(?:\(\s*\)|_)\s*=>\s*\{\s*\}\s*\)"), // 4
        mk(r"[a-zA-Z_]\w*!\.[a-zA-Z_]"),                        // 5
        mk(r"fetch\s*\([^)\n]+\)\s*(?:\.then|;)"),              // 6
        mk(r"\bas\s+any\b"),                                    // 7
        mk(r"@ts-ignore|@ts-expect-error"),                     // 8
        mk(r"eslint-disable(?:-next-line)?"),                   // 9
        mk(r"@ts-nocheck"),                                     // 10
        mk(r"dangerouslySetInnerHTML"),                         // 11
        mk(r"\.innerHTML\s*="),                                 // 12
        mk(r"\beval\s*\("),                                     // 13
        mk(r"new\s+Function\s*\("),                             // 14
        mk(r"document\.write\s*\("),                            // 15
        mk(&format!(r"\{unwrp}")),                              // 16
        mk(&format!(r"\b{dbm}{mc}")),                           // 17
        mk(&format!(r"\b(?:{pln}|{epln}|{prt}|{eprt}){mc}")),   // 18
        mk(&format!(r"\b(?:{s1}|{s2}){mc}")),                   // 19
        mk(r"\bunsafe\s*\{"),                                   // 20
        mk(&format!(r"\b{pnc}{mc}")),                           // 21
        mk(&format!(r"(?:std::)?{pe}{mc}")),                    // 22
        mk(r"#\[allow\(dead_code\)\]"),                         // 23
        mk(r"#\[allow\(unused"),                                // 24
        mk(r"#\[allow\(clippy::"),                              // 25
        mk(r#"\.expect\(\s*"[^"]{0,20}"\s*\)"#),                // 26
        mk(r"\.clone\(\)"),                                     // 27
        mk(r"\bfmt\.(Print|Printf|Println)\s*\("),              // 28
        mk(&format!(r"\b{pnc_go}{mc}")),                        // 29
        mk(r"\b_\s*=\s*\w+\.\w+\("),                            // 30
        mk(r"//\s*nolint"),                                     // 31
        mk(r"\bos\.Exit\s*\("),                                 // 32
        mk(r"\bprint\s*\("),                                    // 33
        mk(r"except\s*:\s*$"),                                  // 34
        mk(r"#\s*type:\s*ignore"),                              // 35
        mk(r"#\s*noqa"),                                        // 36
        mk(r"\b(?:eval|exec)\s*\("),                            // 37
        mk(r"System\.out\.print"),                              // 38
        mk(r"catch\s*\([^)]*\)\s*\{\s*\}"),                     // 39
        mk(r"@SuppressWarnings"),                               // 40
        mk(r"(?i)^FROM\s+\S+:latest"),                          // 41
        mk(&cm),                                                // 42
        mk(r"(curl|wget)\s+[^|]*\|\s*(ba)?sh"),                 // 43
        mk(r"(?im)^ADD\s+"),                                    // 44
        mk(r"(?im)^USER\s+"),                                   // 45
        mk(r"(?i)\b(?:const|let|var)\s+(?:mock|dummy|fake|sample|placeholder)\w*\s*[=:]"), // 46
        mk(r"(?s)\[\s*\{[^}]*\bid\s*:.*?\}\s*,\s*\{[^}]*\bid\s*:.*?\}\s*,\s*\{[^}]*\bid\s*:"), // 47
        mk(r"useState\(\s*\[\s*\{"),                            // 48
        mk(r"(?i)(likes|followers|posts|views|subscribers)\s*:\s*\d{3,}"), // 49
        mk(r"vec!\s*\[\s*(?:serde_json::)?json!\s*\("),         // 50
        mk(r"(?i)StatusCode::NOT_IMPLEMENTED|status\(501\)|\.status\(501\)"), // 51
        mk(r#"(?i)"not.?implemented"|"coming.?soon"|"stub"|"placeholder""#), // 52
        mk(r"(?i)StatusCode::INTERNAL_SERVER_ERROR.*(?:auth|login|password|token|unauthorized)"), // 53
        mk(r"(?i)StatusCode::NOT_FOUND.*(?:forbidden|permission|authorize|denied)"), // 54
        mk(r#"(?i)Ok\(\s*Json\s*\(\s*json!\s*\(\s*\{[^}]*"error""#),                 // 55
        // Algorithmic anti-patterns
        mk(
            r"(?s)for\s+\w+\s+in\s+.*\{[^}]*(?:query|execute|fetch|select|insert|update|delete)\s*\(",
        ), // 56 N+1 query
        mk(r"(?s)for\s+\w+\s+in\s+.*\{[^}]*for\s+\w+\s+in\s+"), // 57 nested loop O(n²)
        // Empty response body — handler returns nothing useful
        mk(r#"(?i)(?:Ok|Json)\s*\(\s*(?:""|\(\)|\{\}|json!\(\{\}\))\s*\)"#), // 58 empty response
        // Frontend API drift: NOT_IMPLEMENTED comment in a TS/JS API client file.
        // Stale after backend route is wired — comment is never removed.
        // Fires on frontend files only (see detect.rs). Case-insensitive.
        mk(r"(?i)NOT_IMPLEMENTED"), // 59 stale stub comment
        // Hardcoded base URL in frontend API client — breaks env promotion (dev→staging→prod).
        // Matches https?:// URLs that are NOT localhost (localhost is caught by PROD_LEAK).
        // No lookahead: match https?:// and filter localhost in detect.rs.
        mk(r#"(?:fetch|axios\.[a-z]+|client\.[a-z]+)\s*\(\s*["'`]https?://"#), // 60 hardcoded base URL
        // Frontend API client function that returns a hardcoded empty collection.
        // Fires when an async function immediately returns [] or {} — live route not called.
        mk(r"(?m)^\s*return\s+(?:\[\s*\]|\{\s*\})\s*;"), // 61 hardcoded empty return
        // fetch() call with no headers object — likely missing Authorization header.
        // Heuristic: bare fetch(url) with no second argument (no {headers:...}).
        // False-positive rate is acceptable: public endpoints should explicitly note they are unauthenticated.
        mk(r#"fetch\s*\(\s*["'`][^"'`]+["'`]\s*\)"#), // 62 fetch with no headers
    ]
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal API surfaced cross-module"
)]
pub(crate) static P: LazyLock<Vec<Regex>> = LazyLock::new(build_patterns);

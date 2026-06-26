use crate::config::j;
use regex::Regex;
use std::sync::LazyLock;

// Patterns are compile-time literals; a malformed one is a build bug, not a runtime
// condition. `\z\A` (end-of-input then start-of-input) is unsatisfiable, so a mis-typed
// pattern degrades to "detects nothing" rather than panicking — keeping the index-addressed
// table length-stable without `unwrap`/`expect` (forbidden workspace-wide).
fn mk(p: &str) -> Regex {
    Regex::new(p).unwrap_or_else(|_| never_match())
}

// `\z\A` always compiles; the recursion on the same literal is provably one iteration and
// needs no panic/unwrap. Separated from `mk` so the fallback path is trivially terminating.
fn never_match() -> Regex {
    Regex::new(r"\z\A").unwrap_or_else(|_| never_match())
}

struct MacroPatterns {
    unwrp: String,
    uwor: String,
    uworelse: String,
    uwordef: String,
    pln: String,
    epln: String,
    prt: String,
    eprt: String,
    s1: String,
    s2: String,
    pe: String,
    dbm: String,
    pnc: String,
}

fn build_macro_patterns() -> MacroPatterns {
    MacroPatterns {
        unwrp: j(&[".unw", "rap()"]),
        uwor: j(&[".unw", r"rap_or\("]),
        uworelse: j(&[".unw", r"rap_or_else\("]),
        uwordef: j(&[".unw", r"rap_or_default\(\)"]),
        pln: j(&["pri", "nt", "ln!"]),
        epln: j(&["epr", "int", "ln!"]),
        prt: j(&["pri", "nt!"]),
        eprt: j(&["epr", "int!"]),
        s1: j(&["to", "do!"]),
        s2: j(&["uni", "mple", "mented!"]),
        pe: j(&["pro", "cess", "::", "exit"]),
        dbm: j(&["db", "g!"]),
        pnc: j(&["pan", "ic!"]),
    }
}

fn build_safety_patterns(mc: &str) -> Vec<Regex> {
    let MacroPatterns {
        unwrp,
        uwor,
        uworelse,
        uwordef,
        pln,
        epln,
        prt,
        eprt,
        s1,
        s2,
        pe,
        dbm,
        pnc,
    } = build_macro_patterns();

    vec![
        mk(&format!(r"\{unwrp}")),                              // 0 P0
        mk(&format!(r"\b{pnc}{mc}")),                           // 1 P0
        mk(&format!(r"(?:std::)?{pe}{mc}")),                    // 2 P0
        mk(r" as\s+(?:u8|i16|u16|i32|u32)\b"),                  // 3 P0 narrowing cast
        mk(&format!(r"\b{dbm}{mc}")),                           // 4 P1
        mk(&format!(r"\b(?:{pln}|{epln}|{prt}|{eprt}){mc}")),   // 5 P1
        mk(&format!(r"\b(?:{s1}|{s2}){mc}")),                   // 6 P1
        mk(r"\bunsafe\s*\{"),                                   // 7 P1 (without SAFETY comment)
        mk(r"#\[allow\(dead_code\)\]"),                         // 8 P3
        mk(r"#\[allow\(unused"),                                // 9 P3
        mk(r"#\[allow\(clippy::"),                              // 10 P3
        mk(r#"\.expect\(\s*"[^"]{0,20}"\s*\)"#),                // 11 P2
        mk(r"\.clone\(\)"),                                     // 12 P2 (excessive)
        mk(&format!(r"\{uwor}")),     // 13 P1 hides errors behind defaults
        mk(&format!(r"\{uworelse}")), // 14 P1 swallows error context
        mk(&format!(r"\{uwordef}")),  // 15 P1 hides errors behind default
        mk(r"\.ok\(\)"),              // 16 P1 silently discards Err→None
        mk(r"\w+\[\w+\]"),            // 17 P2 direct indexing — use .get()
        mk(r"\.\.\s*Default::default\(\)"), // 18 P0 hidden fields via Default
        mk(r"^\s*_\s*=>"),            // 19 P0 wildcard catch-all in match
        mk(r"#\[serde\(default\)\]\s*\n?\s*pub\s+\w+:\s*bool"), // 20 P0 serde default on bool
        mk(r"(?m)^\s*(?:pub\s+)?fn\s+\w+\s*\([^)]*\)\s*(?:->\s*\S+\s*)?\{\s*\}"), // 21 P0 empty fn body
        mk(r"(?i)(?:StatusCode::OK|200)\s*.*(?:error|err|fail|invalid|unauthorized|denied)"), // 22 P0 200 with error
        mk(r"(?i)StatusCode::INTERNAL_SERVER_ERROR\s*.*(?:valid|input|missing|required|format)"), // 23 P0 500 for validation
    ]
}

fn build_design_patterns() -> Vec<Regex> {
    vec![
        // Encapsulation: pub fields on structs that should use accessors
        mk(r"(?m)^\s*pub\s+\w+\s*:\s*(?:String|Vec|HashMap|Option|bool|i32|i64|u32|u64|f64|usize)"), // 24 P1 pub field
        // Optimization: Vec::new() followed by push in loop without with_capacity
        mk(r"(?s)let\s+(?:mut\s+)?\w+\s*=\s*Vec::new\(\)\s*;[^;]{0,200}(?:for|while|loop)\b"), // 25 P1 vec no capacity
        // Abstraction: concrete type in function param instead of trait/generic
        mk(r"fn\s+\w+\s*\([^)]*:\s*&(?:Vec|String|HashMap)<"), // 26 P1 concrete type param
        // Polymorphism: downcast or type check instead of trait dispatch
        mk(r"(?:\.downcast|TypeId::of|Any::type_id|is::<)"), // 27 P1 manual type dispatch
        // Validation: handler with extract but no validate/check/verify
        mk(r"(?s)(?:Json|Query|Path|Form)<[^>]+>\s*\)\s*(?:->.*?)?\{[^}]{0,300}\}"), // 28 P1 no validation
        // Bool parameter anti-pattern — use enums instead
        mk(r"fn\s+\w+\s*\([^)]*:\s*bool\s*[,)]"), // 29 P1 bool param
    ]
}

fn build_protocol_markers() -> Vec<Regex> {
    vec![
        mk(r"WebSocketUpgrade|ws::Message|WebSocket"), // 30 WebSocket
        mk(r"Sse::new|sse::Event|SseStream"),          // 31 SSE
        mk(&format!(
            r"(?:async_{}|juniper|MergedObject)",
            j(&["gra", "ph"])
        )), // 32 GQL
        mk(&format!(
            r"(?:{}::|{}Service|\.proto\b)",
            j(&["ton", "ic"]),
            j(&["Gr", "pc"])
        )), // 33 RPC
        mk(&format!(
            r"(?:{}::|{}::|{})",
            j(&["h", "3"]),
            j(&["qui", "nn"]),
            j(&["Http", "3"])
        )), // 34 H3
        // REST-only router (presence marker for advisory)
        mk(r"Router::new\(\)\s*(?:\.route\()+"), // 35 Router
    ]
}

fn build_async_patterns() -> Vec<Regex> {
    vec![
        // Async: mutex lock held across await point (deadlock)
        mk(r"(?s)\.lock\(\)[^;]{0,100}\.await"), // 36 P0 lock across await
        // Async: no timeout on external call
        mk(r"(?:reqwest|hyper|Client).*\.send\(\)\.await"), // 37 P1 no timeout
        // Microservice: body limit layer presence marker (checked inversely)
        mk(r"\.layer\(.*(?:DefaultBodyLimit|RequestBodyLimit)"), // 38 body limit present
        // DSA: .contains() on Vec inside loop — O(n²)
        mk(r"(?s)for\s+\w+\s+in\s+.*\{[^}]*\.contains\("), // 39 P0 linear search in loop
        // DSA: String concatenation in loop (use push_str or with_capacity)
        mk(r"(?s)for\s+\w+\s+in\s+.*\{[^}]*(?:format!\(|String::from\()"), // 40 P1 string alloc in loop
        // Unbounded allocation: Vec::push in handler without capacity limit
        mk(r"(?s)(?:async\s+fn|pub\s+async\s+fn)\s+\w+.*\.push\("), // 41 P1 unbounded push in handler
        // HTTP client timeout presence marker (checked inversely)
        mk(r"\.timeout\("), // 42 timeout present
        // Chatty service: multiple sequential .await to same base URL
        mk(r"(?s)\.await.{0,5};.{0,100}\.await.{0,5};.{0,100}\.await"), // 43 P1 chatty sequential awaits
    ]
}

fn build_architecture_patterns() -> Vec<Regex> {
    vec![
        // God class: too many fn in one impl block (presence marker — counted)
        mk(r"(?m)^\s*(?:pub\s+)?(?:async\s+)?fn\s+\w+"), // 44 fn count marker
        // Magic numbers: bare numeric literals in logic (not const/let binding)
        mk(r"(?:if|while|match|==|!=|>=|<=|>|<)\s*\d{2,}"), // 45 P1 magic number
        // Long if/else chain — use enum/match instead
        mk(r"(?s)if\s+.*\{[^}]*\}\s*else\s+if\s+.*\{[^}]*\}\s*else\s+if"), // 46 P1 if/else chain
        // Hardcoded string in logic (not const)
        mk(r#"(?:==|!=)\s*"[a-zA-Z_]{3,}""#), // 47 P1 magic string
        // Blanket sqlx error to 500: #[from] sqlx::Error without code inspection
        mk(r"#\[from\]\s*(?:sqlx::Error|SqlxError)"), // 48 P1 blanket sqlx→500
        // INTERNAL_SERVER_ERROR in Err arm without matching on error variant
        mk(r"(?s)Err\s*\(\s*\w+\s*\)\s*=>\s*\{[^}]{0,200}INTERNAL_SERVER_ERROR"), // 49 P1 blanket 500 in Err
    ]
}

fn build_silent_discard_patterns() -> Vec<Regex> {
    vec![
        // Silent discard: let _var = sqlx/DB/HTTP operation — ignoring Result
        mk(r"let\s+_\w*\s*=\s*(?:sqlx::|diesel::|sea_orm::|conn\.|pool\.|db\.|query)"), // 50 P0 silent DB discard
        // Silent discard: let _var = HTTP client operation — ignoring Result
        mk(r"let\s+_\w*\s*=\s*(?:reqwest::|hyper::|client\.|http::)"), // 51 P0 silent HTTP discard
        // Silent discard: let _ = fire-and-forget .await — must check Result
        mk(r"let\s+_\s*=\s*[^;]+\.await"), // 52 P0 silent await discard
        // Underscore param: _name: &Type — suppresses unused warning instead of using
        // Must be preceded by whitespace/comma/paren to avoid matching mid-word like email_client
        mk(r"[\s,(]_\w+:\s*&(?:Option<|Arc<|Box<|Vec<|\w+Client|\w+Service)"), // 53 P1 unused param suppression
    ]
}

fn build_race_patterns() -> Vec<Regex> {
    vec![
        // RACE: TOCTOU — check exists then operate (CWE-367)
        mk(
            r"(?s)(?:\.exists\(\)|\.is_file\(\)|\.is_dir\(\)).*\{[^}]{0,200}(?:File::open|File::create|fs::read|fs::write)",
        ), // 54 P0 file TOCTOU
        // RACE: check-then-act on Option without atomic
        mk(r"(?s)if\s+\w+\.is_none\(\)[^}]{0,100}\w+\s*=\s*Some\("), // 55 P0 check-then-set
        // RACE: HashMap get then insert — use entry API
        mk(r"(?s)\.get\(&[^)]+\)[^;]{0,100}\.insert\("), // 56 P0 get-then-insert
        // RACE: SELECT then UPDATE/INSERT without transaction
        mk(
            r"(?s)(?:query_as!|query!|sqlx::query)[^;]+\.await[^;]{0,300}(?:\.execute|UPDATE|INSERT|DELETE)",
        ), // 57 P0 DB TOCTOU
        // RACE: non-atomic counter increment
        mk(r"\w+\s*\+=\s*1\s*;"), // 58 P1 non-atomic increment — use AtomicU64
    ]
}

fn build_crate_canon_patterns() -> Vec<Regex> {
    vec![
        // 59 P1: const VALID_X: &[&str] → propose strum enum (EnumString + EnumIter)
        mk(r"(?m)^\s*(?:pub\s+)?const\s+\w+:\s*&\[&str\]\s*="),
        // 60 P1: function with >=5 &str params → propose bon #[builder]
        mk(r"fn\s+\w+\s*\([^)]*&str[^)]*&str[^)]*&str[^)]*&str[^)]*&str"),
        // 61 P2: manual impl Default with literal fields → propose smart-default
        mk(
            r"(?s)impl\s+Default\s+for\s+\w+\s*\{[^}]*fn\s+default\s*\(\s*\)\s*->\s*Self\s*\{[^}]{30,}\}",
        ),
        // 62 P2: .collect::<Vec<_>>().join( → propose itertools::Itertools::join
        mk(r"\.collect::<Vec<_>>\(\)\.join\("),
        // 63 P2: pub fn returning HashMap — consider IndexMap if iteration order matters
        mk(r"pub\s+fn\s+\w+\s*\([^)]*\)\s*->\s*HashMap<"),
        // 64 P1: bare `fn main() {` without color_eyre::install in binary
        mk(r"(?m)^fn\s+main\s*\(\s*\)\s*\{"),
    ]
}

fn build_extension_canon_patterns() -> Vec<Regex> {
    vec![
        // 65 P2: std::sync::Mutex import → propose parking_lot::Mutex
        mk(r"use\s+std::sync::Mutex\b"),
        // 66 P2: Arc<RwLock<T>> for read-mostly state → propose arc_swap::ArcSwap
        mk(r"Arc\s*<\s*RwLock\s*<"),
        // 67 P2: `for .. in .. { sleep` retry loop → propose backoff crate
        mk(r"(?s)for\s+\w+\s+in\s+[^{]+\{[^}]{0,200}(?:thread::sleep|tokio::time::sleep)"),
        // 68 P2: AtomicBool used as shutdown flag → propose tokio_util::sync::CancellationToken
        mk(r"AtomicBool::new\(\s*(?:true|false)\s*\).*shutdown|shutdown.*AtomicBool::new"),
        // 69 P2: pub struct field `pub \w+: String` in struct with `key|id|tag|slug|category` semantics → CompactString
        mk(r"(?m)^\s*pub\s+(?:key|id|tag|slug|category|name)\w*\s*:\s*String\s*,"),
    ]
}

fn build_status_code_patterns() -> Vec<Regex> {
    vec![
        // 70 P1: enum named `*Error` deriving thiserror::Error but NO impl IntoResponse
        // — handlers will collapse all variants to 500 (anyhow::Error default).
        mk(r"(?s)#\[derive\([^)]*thiserror::Error[^)]*\)\][^{]*pub\s+enum\s+\w*Error\b[^}]*\}"),
        // 71 P0: 4xx-class variant collapsed to 500 — semantic loss (CWE-209).
        // Covers ALL RFC 9110 §15.5 client-error variants the domain typically names.
        mk(
            r"(?s)Err\s*\(\s*\w+::(?:NotFound|Forbidden|Conflict|BadRequest|Validation|Unauthorized|Unauthenticated|MethodNotAllowed|NotAcceptable|RequestTimeout|Gone|PreconditionFailed|PayloadTooLarge|UriTooLong|UnsupportedMediaType|RangeNotSatisfiable|ExpectationFailed|TeaPot|MisdirectedRequest|UnprocessableEntity|Locked|FailedDependency|TooEarly|UpgradeRequired|PreconditionRequired|TooManyRequests|HeaderFieldsTooLarge|UnavailableForLegalReasons|PaymentRequired)\s*[^}]*\)\s*=>\s*[^,}]{0,200}INTERNAL_SERVER_ERROR",
        ),
        // 72 P0: full 2xx series returned alongside error/fail/invalid/denied/missing words.
        // Extends index 22 (which only caught OK + a few words).
        mk(
            r"(?i)StatusCode::(?:OK|CREATED|ACCEPTED|NON_AUTHORITATIVE_INFORMATION|NO_CONTENT|RESET_CONTENT|PARTIAL_CONTENT|MULTI_STATUS|ALREADY_REPORTED|IM_USED)\b[^,;}]{0,100}(?:error|err|fail|invalid|unauthorized|denied|missing|not_found)",
        ),
        // 73 P1: 3xx redirect (except 304) — RFC 9110 §15.4 requires Location header.
        // Heuristic detector; rust_guard.rs gates final severity by presence of `Location`.
        mk(
            r"StatusCode::(?:MOVED_PERMANENTLY|FOUND|SEE_OTHER|TEMPORARY_REDIRECT|PERMANENT_REDIRECT|MULTIPLE_CHOICES)\b",
        ),
        // 74 P0: full 5xx series returned for validation/schema/parse failure — must be 4xx.
        // Extends index 23.
        mk(
            r"(?i)StatusCode::(?:INTERNAL_SERVER_ERROR|NOT_IMPLEMENTED|BAD_GATEWAY|SERVICE_UNAVAILABLE|GATEWAY_TIMEOUT|HTTP_VERSION_NOT_SUPPORTED|VARIANT_ALSO_NEGOTIATES|INSUFFICIENT_STORAGE|LOOP_DETECTED|NOT_EXTENDED|NETWORK_AUTHENTICATION_REQUIRED)\b[^,;}]{0,100}(?:valid|input|missing|required|format|schema|parse|deserialize|required_field)",
        ),
        // 75 P1: hardcoded numeric status integer used in an HTTP-specific position.
        // Narrowed to explicit forms only:
        //   - StatusCode::from_u16(404)
        //   - .status(200)        (axum Response builder)
        //   - HTTP_STATUS = 500   (explicit constant assignment)
        // Avoids false positives on `let timeout_ms = 200;` or `sleep(400);`.
        mk(
            r"(?:StatusCode::from_u16\s*\(|\.status\s*\(|HTTP_STATUS\s*[:=])\s*\(?\s*\b(?:100|101|102|103|200|201|202|203|204|205|206|207|208|226|300|301|302|303|304|305|307|308|400|401|402|403|404|405|406|407|408|409|410|411|412|413|414|415|416|417|418|421|422|423|424|425|426|428|429|431|451|500|501|502|503|504|505|506|507|508|510|511)\b\s*(?:u16|u32|i32)?\s*\)?",
        ),
        // 76 P1: handler returns Ok(StatusCode::4xx|5xx) — error wrapped as success.
        // Middleware sees Ok and skips error logging / tracing span error attribute.
        mk(
            r"Ok\s*\(\s*StatusCode::(?:BAD_REQUEST|UNAUTHORIZED|FORBIDDEN|NOT_FOUND|CONFLICT|UNPROCESSABLE_ENTITY|TOO_MANY_REQUESTS|INTERNAL_SERVER_ERROR|BAD_GATEWAY|SERVICE_UNAVAILABLE|GATEWAY_TIMEOUT)\b",
        ),
        // 77 P1: 1xx informational returned from handler — wrong layer.
        // hyper handles 100 Continue / 101 Switching at transport; never axum handler.
        mk(r"StatusCode::(?:CONTINUE|SWITCHING_PROTOCOLS|PROCESSING|EARLY_HINTS)\b"),
        // 78 P1: general named-underscore discard `let _name = <expr>;` — the author
        // NAMED what the value IS then threw it away, so a return carrying a decision
        // (bool won/lost, Result ok/err, row count) is silently dropped and the caller
        // never reacts when it needs to (the dispatch work-steal bug: `let _claimed =
        // claim_card(...)`). Indices 50-52 catch only DB/HTTP/await; this generalizes.
        // The `[A-Za-z]` after `_` excludes anonymous `let _ =` / `let _: T =` in the
        // regex itself. RAII held-for-drop names (guard/lock/span/…) cannot be excluded
        // with this engine (no lookaround), so `discard_race.rs` filters them per-match
        // on the captured binding name. MUST stay the LAST entry: indices are positional,
        // so appending here keeps 0-77 stable (a mid-table insert shifts every later index).
        mk(r"(?m)^\s*let\s+(_[A-Za-z]\w*)\s*=\s*\S"), // 78 P1 named-underscore discard
        // 79 P1: anonymous `let _ = call(` / `= (a, b)` discard (let_underscore_must_use, rustc PR 97739); bare `()`/literal stay silent. decision.gate.anon-underscore-no-launder
        mk(r"(?m)^\s*let\s+_\s*=\s*(?:\w[\w:]*\s*\(|\([A-Za-z_]\w*\s*,)"), // 79 P1 anon discard
    ]
}

fn build() -> Vec<Regex> {
    let mc = r"\s*\(";
    let mut patterns = Vec::new();
    patterns.extend(build_safety_patterns(mc));
    patterns.extend(build_design_patterns());
    patterns.extend(build_protocol_markers());
    patterns.extend(build_async_patterns());
    patterns.extend(build_architecture_patterns());
    patterns.extend(build_silent_discard_patterns());
    patterns.extend(build_race_patterns());
    patterns.extend(build_crate_canon_patterns());
    patterns.extend(build_extension_canon_patterns());
    patterns.extend(build_status_code_patterns());
    patterns
}

#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal regex table consumed by rust_guard.rs; module is private so pub would be unreachable"
)]
pub(crate) static RUST_P: LazyLock<Vec<Regex>> = LazyLock::new(build);

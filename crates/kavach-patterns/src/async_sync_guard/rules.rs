use super::types::AsyncSeverity;
use regex::Regex;
use std::sync::LazyLock;

pub(super) struct Rule {
    pub(super) checker: fn(&str) -> bool,
    pub(super) sev: AsyncSeverity,
    pub(super) pattern: &'static str,
    pub(super) fix: &'static str,
}

pub(super) static RULES: LazyLock<Vec<Rule>> = LazyLock::new(|| {
    vec![
        Rule {
            checker: |line| line.contains("std::sync::Mutex"),
            sev: AsyncSeverity::P0Block,
            pattern: "std::sync::Mutex in async context",
            fix: "Use tokio::sync::Mutex — std::sync::Mutex held across .await deadlocks the runtime.",
        },
        Rule {
            checker: |line| line.contains("std::thread::sleep"),
            sev: AsyncSeverity::P0Block,
            pattern: "std::thread::sleep blocks runtime",
            fix: "Use tokio::time::sleep — blocking sleep starves the async runtime.",
        },
        Rule {
            checker: |line| {
                line.contains("std::fs::")
                    && (line.contains("read")
                        || line.contains("write")
                        || line.contains("File::open")
                        || line.contains("File::create"))
            },
            sev: AsyncSeverity::P1Advisory,
            pattern: "std::fs blocks async runtime",
            fix: "Use tokio::fs or wrap in tokio::task::spawn_blocking.",
        },
        Rule {
            checker: |line| line.contains("tokio::spawn"),
            sev: AsyncSeverity::P1Advisory,
            pattern: "tokio::spawn JoinHandle discarded",
            fix: "Bind JoinHandle: `let h = tokio::spawn(...)` — track errors and join on shutdown.",
        },
        Rule {
            checker: |line| line.contains("select!") && line.contains(".send("),
            sev: AsyncSeverity::P1Advisory,
            pattern: "non-cancel-safe send in select! branch",
            fix: "mpsc::send is NOT cancel-safe — losing this branch loses the message. Use try_send or move to spawned task.",
        },
        Rule {
            checker: |line| {
                line.contains("select!")
                    && (line.contains(".write(") || line.contains(".write_all("))
            },
            sev: AsyncSeverity::P1Advisory,
            pattern: "non-cancel-safe write in select! branch",
            fix: "AsyncWriteExt::write_all is NOT cancel-safe — partial writes leave broken state. Move to spawned task.",
        },
        Rule {
            checker: |line| line.contains("tokio::sync::mpsc::unbounded_channel"),
            sev: AsyncSeverity::P1Advisory,
            pattern: "unbounded_channel — unbounded queue → unbounded p99",
            fix: "Use mpsc::channel(N) with explicit backpressure. Kernel-bypass tier: N ≤ 1024.",
        },
        Rule {
            checker: |line| line.contains("std::sync::mpsc::"),
            sev: AsyncSeverity::P0Block,
            pattern: "std::sync::mpsc — sync channel in async context",
            fix: "Use tokio::sync::mpsc — std::mpsc::recv() blocks the runtime thread.",
        },
        Rule {
            checker: |line| line.contains(".lock()") && line.contains(".await"),
            sev: AsyncSeverity::P1Advisory,
            pattern: "lock().await held across suspension — contention amplifier",
            fix: "Kernel-bypass tier: scope the guard to a sync block, drop before .await. Or use parking_lot::Mutex if guard never crosses .await.",
        },
        Rule {
            checker: |line| {
                line.trim_start().starts_with("println!")
                    || line.trim_start().starts_with("eprintln!")
            },
            sev: AsyncSeverity::P1Advisory,
            pattern: "println!/eprintln! — line-buffered sync I/O",
            fix: "Use tracing::info!/error! — println! takes a stdout lock and syscalls per call (~µs each).",
        },
        Rule {
            checker: |line| line.contains("Regex::new("),
            sev: AsyncSeverity::P1Advisory,
            pattern: "Regex::new in fn body — recompile per call",
            fix: "Hoist to `static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(...).expect(\"static\"));`",
        },
        Rule {
            checker: |line| {
                line.contains("std::net::TcpStream")
                    || line.contains("std::net::TcpListener")
                    || line.contains("std::net::UdpSocket")
            },
            sev: AsyncSeverity::P0Block,
            pattern: "std::net — blocking syscall in async context",
            fix: "Use tokio::net::* — std::net::TcpStream::read blocks the runtime worker.",
        },
        Rule {
            checker: |line| line.contains("reqwest::blocking"),
            sev: AsyncSeverity::P0Block,
            pattern: "reqwest::blocking inside async fn",
            fix: "Use reqwest::Client (async) — reqwest::blocking spawns a private runtime and blocks.",
        },
    ]
});

pub(super) struct HotPathRule {
    pub(super) checker: fn(&str) -> bool,
    pub(super) pattern: &'static str,
    pub(super) fix: &'static str,
}

pub(super) static HOT_PATH_RULES: LazyLock<Vec<HotPathRule>> = LazyLock::new(|| {
    vec![
        HotPathRule {
            checker: |line| line.contains("format!("),
            pattern: "format!() in hot path — heap alloc + UTF-8 validation",
            fix: "Pre-allocate `String::with_capacity(N)` and `write!()` into it, or use `itoa`/`ryu` for numeric formatting.",
        },
        HotPathRule {
            checker: |line| line.contains("Vec::new()") || line.contains("vec!["),
            pattern: "Vec::new()/vec![] in hot path — alloc on each call",
            fix: "Use `Vec::with_capacity(N)` or reuse a pre-allocated buffer across calls.",
        },
        HotPathRule {
            checker: |line| {
                line.contains("String::new()")
                    || line.contains(".to_string()")
                    || line.contains(".to_owned()")
            },
            pattern: "String allocation in hot path",
            fix: "Use `&str` or `Cow<'_, str>` — avoid heap allocation in p99-critical code.",
        },
        HotPathRule {
            checker: |line| line.contains("Box::new("),
            pattern: "Box::new in hot path — heap alloc",
            fix: "Use stack allocation or a pre-allocated arena (e.g. bumpalo).",
        },
        HotPathRule {
            checker: |line| line.contains("Arc::clone(") || line.contains(".clone()"),
            pattern: "clone()/Arc::clone in hot path — atomic RMW ~10ns or heap copy",
            fix: "Pass `&T` instead, or move holdership. Arc::clone is an atomic increment — costly in tight loops.",
        },
        HotPathRule {
            checker: |line| {
                line.contains("HashMap::new()") || line.contains("HashMap::with_capacity(")
            },
            pattern: "std HashMap (SipHash, ~30ns/op) in hot path",
            fix: "Use ahash::AHashMap or hashbrown::HashMap with FxHasher — 4-10x faster on non-adversarial input.",
        },
        HotPathRule {
            checker: |line| line.contains(".collect::<Vec<"),
            pattern: ".collect::<Vec<_>>() in hot path — intermediate alloc",
            fix: "Chain iterators directly to the consumer, or `.collect_into(&mut buf)` (nightly) / `extend(&mut buf, ...)`.",
        },
        HotPathRule {
            checker: |line| {
                line.contains("serde_json::from_str") || line.contains("serde_json::to_string")
            },
            pattern: "serde_json::{from_str,to_string} in hot path — heap-allocating",
            fix: "Use `from_slice`/`to_writer` with a reused `Vec<u8>` buffer, or simd-json/sonic-rs for sub-µs parse.",
        },
    ]
});

#[inline]
pub(super) fn is_hot_path_fn(file_path: &str, fn_name: &str, attrs_above: &str) -> bool {
    let stem = fn_name.trim_start_matches("r#");
    stem.starts_with("handle_")
        || stem.starts_with("process_")
        || stem.starts_with("tick_")
        || stem.starts_with("on_event_")
        || stem.starts_with("poll_")
        || stem == "call"
        || attrs_above.contains("#[inline(always)]")
        || attrs_above.contains("#[axum::debug_handler]")
        || file_path.contains("/hot_path/")
        || file_path.contains("/fast_path/")
        || file_path.contains("/handlers/")
}

pub(super) static LOOP_KEYWORD: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"\b(?:for|while)\s+\w|\bloop\s*\{").ok());

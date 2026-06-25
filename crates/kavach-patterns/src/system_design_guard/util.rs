use regex::Regex;
use std::sync::LazyLock;

static R0: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"(?:reqwest::Client::(?:new|builder)|reqwest::ClientBuilder::new)\s*\(\s*\)").ok()
});
static R1: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"axios\.create\s*\(\s*\{[^}]*\}").ok());
static R2: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"\bfetch\s*\(\s*[`'"][^`'"]+[`'"]\s*\)"#).ok());
static R3: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?:retry|backoff)[^{]*\{[^}]*sleep\s*\([^)]*\)[^}]*\}").ok());
static R4: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"for\s+\w+\s+in\s+[^{]+\{[^}]*\.await[^}]*\.await").ok());
static R5: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"(?:Cache::new|moka::(?:future|sync)::Cache|cached::|stretto::|cacache::)\s*[(<]")
        .ok()
});
static R6: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"async\s+fn\s+(?:charge|process_payment|create_payment|transfer_funds|transfer_money|pay|debit|withdraw)(?:_|\()").ok()
});
static R7: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"(?:tokio::sync::mpsc::unbounded_channel|unbounded_channel|crossbeam::channel::unbounded)\s*\(\s*\)").ok()
});
static R8: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"if\s+(?:status|code|err|e)\.[\w_]*(?:is_4|status_code\(\)\s*==\s*4|400|401|403|404|422)[^}]*\{[^}]*retry").ok()
});
static R9: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?:consume|process_message|handle_event|on_message)\s*\(").ok());
static R10: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"loop\s*\{[^}]*\.await[^}]*sleep\s*\(").ok());
static R11: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"async\s+fn[\s\S]*?futures::executor::block_on").ok());
static R12: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"(?:sleep|delay|wait)\s*\(\s*Duration::from_(?:secs|millis)\s*\(\s*\d+\s*\)\s*\)\s*\.\s*await\s*;[\s\S]{0,200}\.await").ok()
});
static R13: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?:Arc<Mutex<|Arc<RwLock<|Arc<DashMap<)").ok());
static R14: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(
        r"\.(?:get|post|put|delete|send|call)\s*\([^)]*\)[^;]*\.\s*await[^;]*\.\s*unwrap\s*\(\s*\)",
    )
    .ok()
});
static OVERFETCH: LazyLock<Option<Regex>> = LazyLock::new(|| {
    let s = format!("{}{}", "S", "ELECT");
    let star = "[*]";
    let pat = format!(r#"(?:query|fetch|prepare)\s*\(\s*[`'"]\s*{s}\s*{star}"#);
    Regex::new(&pat).ok()
});

pub(super) fn get_patterns() -> [&'static Option<Regex>; 16] {
    [
        &R0, &R1, &R2, &R3, &R4, &R5, &R6, &R7, &R8, &R9, &R10, &R11, &R12, &R13, &R14, &OVERFETCH,
    ]
}

pub(super) fn is_code_extension(path: &str) -> bool {
    let p = path.to_lowercase();
    matches!(p.rsplit('.').next(), Some(ext) if matches!(ext, "rs" | "ts" | "js" | "go" | "py"))
}

pub(super) fn is_service_file(path: &str) -> bool {
    let p = path.to_lowercase();
    if !is_code_extension(&p) {
        return false;
    }
    p.contains("/handlers/")
        || p.contains("/services/")
        || p.contains("/api/")
        || p.contains("/server/")
        || p.contains("/backend/")
        || p.contains("/worker/")
        || p.contains("/consumer/")
        || p.contains("/producer/")
        || p.contains("/queue/")
        || p.ends_with("server.rs")
        || p.ends_with("service.rs")
        || p.ends_with("handler.rs")
        || p.ends_with("worker.rs")
        || p.ends_with("consumer.rs")
        || p.ends_with("client.rs")
}

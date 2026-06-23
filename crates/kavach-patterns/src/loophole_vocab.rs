//! Tech-stack-agnostic loophole-lens vocabulary: floor + graph overlay.
//!
//! The compiled `Default` is a THIN, CROSS-LANGUAGE floor — each dimension carries
//! a handful of universal concept markers spanning Rust/TS/JS/Python/Go/Java/C/C++,
//! never one stack's tokens. A project's `gate.loophole_vocab` DB row ADDS the rich
//! marker set + new dimensions on top (research-refreshable, no rebuild). The graph
//! ADDS, never replaces: the floor always detects, so a DB outage degrades to the
//! compiled baseline (fail-closed). SOURCE: decision.loophole-mistake-umbrella +
//! decision.w5 (a security detector's vocabulary stays in-binary). Dimension taxonomy
//! from OWASP Top 10:2025 + CWE Top 25 2025.

/// One risk dimension of the loophole taxonomy.
///
/// Carries its short `label` (the heading taxonomy), a Brain-OS `lens_query` (steers
/// lens retrieval), and the agnostic `markers` that fire it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct DimensionRule {
    /// Short kebab label naming the dimension (authz / injection / xss / …).
    pub label: String,
    /// Brain-OS retrieval query for surface-specific lenses on this dimension.
    pub lens_query: String,
    /// Cross-language substrings whose presence fires this dimension.
    pub markers: Vec<String>,
}

/// The loophole-lens vocabulary AS DATA: a thin agnostic floor + additive overlay.
///
/// `#[serde(default)]` fills each field the row omits from the compiled floor, so a
/// partial/malformed overlay degrades to the full floor. `#[non_exhaustive]` keeps
/// the struct additive across versions.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct LoopholeVocab {
    /// Per-dimension rules (label + lens query + agnostic markers).
    pub dimensions: Vec<DimensionRule>,
}

impl LoopholeVocab {
    /// Flatten every dimension's markers into the trigger set (the scope half of the
    /// detector — content touching any of these warrants the adversarial prompt).
    #[must_use]
    pub fn trigger_markers(&self) -> Vec<&str> {
        self.dimensions
            .iter()
            .flat_map(|d| d.markers.iter().map(String::as_str))
            .collect()
    }
}

impl Default for LoopholeVocab {
    fn default() -> Self {
        Self {
            dimensions: floor_dimensions(),
        }
    }
}

/// `Some(label)` for the dimension owning `token`, else `None`.
///
/// Matches the real scanner's semantics: a floor marker fires when it is a SUBSTRING
/// of the scanned token (`axios` fires on `axios.get`), so a thin floor marker covers
/// a family of call sites across languages. First-match-wins over dimension order
/// (floor dims precede overlay-appended ones).
#[must_use]
pub fn dimension_for_marker(vocab: &LoopholeVocab, token: &str) -> Option<String> {
    vocab
        .dimensions
        .iter()
        .find(|d| d.markers.iter().any(|m| token.contains(m.as_str())))
        .map(|d| d.label.clone())
}

/// Distinct dimension labels for the fired `markers`, comma-joined, dedup-preserving
/// order. Empty / all-unknown ⇒ `general` (the catch-all floor). SOURCE:
/// decision.loophole-surface-heading-dynamic.
#[must_use]
pub fn fired_dimensions(vocab: &LoopholeVocab, markers: &[&str]) -> String {
    let mut seen: Vec<String> = Vec::new();
    for &m in markers {
        if let Some(label) = dimension_for_marker(vocab, m) {
            if !seen.contains(&label) {
                seen.push(label);
            }
        }
    }
    if seen.is_empty() {
        return "general".to_owned();
    }
    seen.join(", ")
}

/// Build one floor dimension from static cross-language marker slices.
fn dim(label: &str, lens_query: &str, markers: &[&str]) -> DimensionRule {
    DimensionRule {
        label: label.to_owned(),
        lens_query: lens_query.to_owned(),
        markers: markers.iter().map(|s| (*s).to_owned()).collect(),
    }
}

/// The compiled, tech-agnostic FLOOR — thin per dimension; the graph adds the rest.
/// Each marker list mixes languages on purpose so one lens fires regardless of stack.
fn floor_dimensions() -> Vec<DimensionRule> {
    vec![
        dim("authz", "authentication authorization session token idor loophole lens",
            &["auth", "token", "session", "permission", "authorize", "current_user"]),
        dim("idor", "authorization bypass user-controlled-key idor ownership loophole lens",
            &["find_by_id", "findById", "get_object_or_404", "params[:id]", "request.user"]),
        dim("crypto", "crypto key-management nonce-reuse weak-algorithm loophole lens",
            &["encrypt", "decrypt", "cipher", "hmac", "md5", "sha1", "random("]),
        dim("money", "money precision rounding idempotency double-spend loophole lens",
            &["payment", "balance", "transfer", "amount", "BigDecimal", "Decimal"]),
        dim("concurrency", "concurrency race deadlock lost-update loophole lens",
            &["lock", "mutex", "atomic", "synchronized", "goroutine", "threading", "race"]),
        dim("persistence", "persistence durability partial-write transaction loophole lens",
            &["transaction", "commit", "persist", "BEGIN", "rollback"]),
        dim("injection", "injection sql command template parameterization loophole lens",
            &["sqlx::query", "execute(", "os.system", "child_process", "Runtime.exec",
              "exec.Command", "eval(", "shell"]),
        dim("ssrf", "ssrf outbound-request url-validation dns-rebinding loophole lens",
            &["reqwest::get", "fetch(", "axios", "requests.get", "urllib", "http.Get",
              "HttpClient", "webhook"]),
        dim("xss", "xss output-encoding html-escaping dom-sink loophole lens",
            &["innerHTML", "dangerouslySetInnerHTML", "v-html", "dangerous_inner_html",
              "render_template", "|safe", "Markup("]),
        dim("csrf", "csrf state-change samesite anti-forgery-token loophole lens",
            &["csrf", "samesite", "@csrf_exempt", "X-CSRF"]),
        dim("deserialization", "deserialization untrusted-input gadget loophole lens",
            &["deserialize", "from_str", "pickle.load", "ObjectInputStream", "yaml.load",
              "JSON.parse", "Marshal"]),
        dim("path-traversal", "path-traversal symlink canonicalization loophole lens",
            &["canonicalize", "../", "os.path.join", "File(", "readFile", "open("]),
        dim("memory-safety", "memory-safety use-after-free buffer-overflow oob loophole lens",
            &["unsafe", "memcpy", "strcpy", "get_unchecked", "transmute", "unsafe.Pointer",
              "malloc", "free("]),
        dim("integer-overflow", "integer-overflow truncation wrapping-cast loophole lens",
            &["wrapping_", " as u", " as i", "(int)", "parseInt", "Number("]),
        dim("resource-exhaustion", "resource-exhaustion dos unbounded backpressure loophole lens",
            &["unbounded", "while True", "loop {", "recursion", "read_to_end", "readAll"]),
        dim("upload", "file-upload unrestricted-type size-limit content-type loophole lens",
            &["multipart", "FileUpload", "request.files", "MultipartFile", "save("]),
        dim("supply-chain", "supply-chain dependency integrity unpinned-version loophole lens",
            &["git dependency", "latest", "curl | sh", "npm install", "pip install"]),
        dim("information-leak", "information-leak secret-in-log error-detail pii loophole lens",
            &["println!", "console.log", "printStackTrace", "logging.debug", "var_dump"]),
        dim("logging", "security-logging audit-event alerting missing-log loophole lens",
            &["audit_log", "security_event", "auth_failure", "login_attempt"]),
    ]
}

#[cfg(test)]
#[path = "loophole_vocab_test.rs"]
mod tests;

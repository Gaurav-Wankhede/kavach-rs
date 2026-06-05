use regex::Regex;
use std::sync::LazyLock;

const LAZY_VERIFY_COUNT: &str = concat!(
    r"(?i)\b\d+\+?(?:\s+\w+){0,2}?\s+(?:files?|components?|lines?|handlers?|routes?)\b",
    r"|\b(?:files?|components?|lines?|handlers?|routes?)\b[\s:]*\d+\+?\b",
    r"|\bwc\s+-l\b|\bfound\s+\d{2,}\b",
);
const LAZY_VERIFY_CLAIM: &str = concat!(
    r"(?i)\b(?:fully\s+)?(?:wired|implemented|complete)\b",
    r"|\bproduction[\s-]ready\b",
);
const LAZY_VERIFY_EVIDENCE: &str = concat!(
    r"(?i)\.(?:rs|ts|tsx|js|jsx|py|go|vue|svelte):|\bat\s+line\b|\bon\s+line\b|```",
    r"|\b(?:the\s+code\s+shows|reading\s+the\s+file|read\s+the\s+component)\b",
    r"|\bverified\s+the\s+api\b|\bchecked\s+the\s+fetch\b|\bthe\s+import\s+shows\b",
    r"|\bfetch\(|\bapi\.|\bendpoint:|\bcalls?\s+(?:backend|api)\b",
    r"|\b(?:imports?\s+from|uses?\s+api)\b",
);

const INFER_EFFECT: &str = concat!(
    r"(?i)\b(?:was|is|got)\s+(?:created|inserted|written|stored|applied)\b",
    r"|\brow\s+(?:exists|is\s+there)\b|\baccount\s+(?:was\s+created|exists)\b",
    r"|\bthe\s+insert\s+completed\b|\bis\s+verified\b|\bnow\s+exists\b",
    r"|\b(?:proves|confirms|means|guarantees|implies)\s+the\b",
    r"|\bis\s+(?:the\s+proof|present|there|in\s+the|stored)\b|\bpersists\b",
);
const INFER_CONNECTIVE: &str = concat!(
    r"(?i)\bso\s+(?:the|a\s+clean)\b|\btherefore\s+the\b|\bwhich\s+proves\b",
    r"|\b(?:is\s+impossible|cannot\s+fail)\s+without\b|\bmust\s+have\b",
    r"|\b(?:succeeded|passed|installed|compiled|ran\s+clean)\s+so\b",
    r"|\bexit\s*0\s+so\b|\bno\s+error\s+so\b",
    r"|\bsince\s+(?:the|it)\b|\bbecause\s+(?:the|it)\b|\bas\s+it\s+ran\b",
    r"|\bgiven\s+that\b|\bcompleted\s+(?:without\s+error|cleanly)\b",
    r"|\bexited\s+cleanly\b|\bwithout\s+error,\b|\bran\s+clean,\b",
);
const INFER_ARTIFACT: &str = concat!(
    r"(?i)\bselect\s|\bquery\s+result\b|\brows?\s+affected\b|\b\d+\s+rows?\b",
    r"|\breturned\b|\bstdout:|\boutput:|```|\.rs:",
    r"|\btest\s+result:\s+ok\b|\bpassed;\s+0\s+failed\b|\bgit\s+diff\b|\brg\s",
    r"|\bi\s+(?:ran|queried)\b|\bthe\s+(?:query|select)\s+(?:shows|returned)\b",
    r"|\bverified\s+by\s+running\b",
);

static LAZY_COUNT_RE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(LAZY_VERIFY_COUNT));
static LAZY_CLAIM_RE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(LAZY_VERIFY_CLAIM));
static LAZY_EVID_RE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(LAZY_VERIFY_EVIDENCE));
static INFER_EFFECT_RE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(INFER_EFFECT));
static INFER_CONN_RE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(INFER_CONNECTIVE));
static INFER_ARTIFACT_RE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(INFER_ARTIFACT));

/// # Errors
/// Returns [`regex::Error`] only if a compile-time-constant pattern fails to
/// compile — unreachable at runtime since the patterns are static literals.
pub fn detect_lazy_verification_claim(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    if !LAZY_COUNT_RE.as_ref().map_err(Clone::clone)?.is_match(msg) {
        return Ok(false);
    }
    if !LAZY_CLAIM_RE.as_ref().map_err(Clone::clone)?.is_match(msg) {
        return Ok(false);
    }
    let has_evidence = LAZY_EVID_RE.as_ref().map_err(Clone::clone)?.is_match(msg);
    Ok(!has_evidence)
}

/// # Errors
/// Returns [`regex::Error`] only if a compile-time-constant pattern fails to
/// compile — unreachable at runtime since the patterns are static literals.
pub fn detect_inference_as_evidence(msg: &str) -> Result<bool, regex::Error> {
    if msg.is_empty() {
        return Ok(false);
    }
    if !INFER_EFFECT_RE
        .as_ref()
        .map_err(Clone::clone)?
        .is_match(msg)
    {
        return Ok(false);
    }
    if !INFER_CONN_RE.as_ref().map_err(Clone::clone)?.is_match(msg) {
        return Ok(false);
    }
    let has_artifact = INFER_ARTIFACT_RE
        .as_ref()
        .map_err(Clone::clone)?
        .is_match(msg);
    Ok(!has_artifact)
}

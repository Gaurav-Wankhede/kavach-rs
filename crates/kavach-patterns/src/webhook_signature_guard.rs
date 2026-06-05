// split: Webhook signature guard. Hard-block on parse-before-verify, advisory on missing nonce/replay.
//
// [RCA]
// symptom:    webhook handler parses request body before verifying HMAC signature → forgery / replay attacks
// repro:      Stripe webhook handler calls serde_json::from_slice(body) before stripe-signature header check
// why1:       no gate flags webhook handlers that deserialize before verifying HMAC
// why2:       handler-shape detection requires path heuristics + body/signature ordering
// why3:       invariant violated — never trust webhook payload until signature is validated
// why4:       Stripe/Github/Twilio docs all flag this as #1 webhook bug class; payment fraud vector
// why5:       missing webhook-shape detection layer
// root_cause: no webhook_signature_guard module
// class:      knowledge_gap + security
// blast_radius: every Rust handler under /webhook/ or /webhooks/ path
// research:   https://stripe.com/docs/webhooks/signatures
//             https://docs.github.com/en/webhooks/using-webhooks/validating-webhook-deliveries
// fix_strategy: 4-pattern P0 module on webhook handler files; wire into pre_write_guards.rs P0 path

use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "cross-crate match in pre_write_guards"
)]
pub enum WhSeverity {
    P0Block,
    P1Advisory,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct WhViolation {
    pub severity: WhSeverity,
    pub pattern: &'static str,
    pub fix: &'static str,
}

fn init_patterns() -> Vec<Regex> {
    [
        r"(?s)serde_json::from_(?:slice|str|reader)\s*\([^)]*\)[^}]{0,2000}?(?:verify_signature|verify_hmac|stripe::Webhook::construct_event|hmac_verify|verify_webhook)",
        r"\bif\s+\w+\s*==\s*\w+\b",
        r"(?s)(?:async\s+)?fn\s+\w*webhook\w*\s*\([^)]*\)[^{]*\{[^}]{0,3000}\}",
        r"(?i)(?:stripe-signature|x-hub-signature|x-twilio-signature|x-shopify-hmac-sha256|x-paypal-transmission-sig|x-slack-signature)",
    ]
    .iter()
    .filter_map(|p| Regex::new(p).ok())
    .collect()
}

static PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(init_patterns);

fn is_webhook_file(path: &str, content: &str) -> bool {
    if !std::path::Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
    {
        return false;
    }
    let p = path.to_ascii_lowercase();
    p.contains("/webhook/")
        || p.contains("/webhooks/")
        || p.ends_with("/webhook.rs")
        || p.ends_with("/webhooks.rs")
        || content.contains("Stripe-Signature")
        || content.contains("X-Hub-Signature")
        || content.contains("X-Twilio-Signature")
        || content.contains("stripe::Webhook")
}

#[must_use]
pub fn detect(file_path: &str, content: &str) -> Vec<WhViolation> {
    if !is_webhook_file(file_path, content) {
        return vec![];
    }
    if crate::file_types::is_test_file(file_path) {
        return vec![];
    }
    let mut v = Vec::new();
    if PATTERNS.first().is_some_and(|p| p.is_match(content)) {
        v.push(WhViolation { severity: WhSeverity::P0Block,
            pattern: "parse-before-verify",
            fix: "serde_json::from_* called before signature verification. Refuse — verify HMAC on raw bytes FIRST, then deserialize." });
    }
    if PATTERNS.get(2).is_some_and(|p| p.is_match(content))
        && PATTERNS.get(1).is_some_and(|p| p.is_match(content))
        && (content.contains("signature") || content.contains("hmac") || content.contains("hash"))
    {
        v.push(WhViolation { severity: WhSeverity::P0Block,
            pattern: "non-constant-time-compare",
            fix: "Webhook signature compared with == (variable-time). Use subtle::ConstantTimeEq or hmac::Mac::verify_slice for timing-safe equality." });
    }
    if PATTERNS.get(3).is_some_and(|p| p.is_match(content))
        && !content.contains("timestamp")
        && !content.contains("Timestamp")
        && !content.contains("replay")
        && !content.contains("nonce")
    {
        v.push(WhViolation { severity: WhSeverity::P1Advisory,
            pattern: "no-replay-window",
            fix: "Webhook handler verifies signature but does not enforce timestamp/nonce replay window. Reject deliveries older than 5 minutes." });
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_before_verify_blocked() {
        let src = "use stripe;\nasync fn handle_webhook(body: &[u8], sig: &str) {\n    let evt: serde_json::Value = serde_json::from_slice(body).unwrap();\n    stripe::Webhook::construct_event(body, sig, \"whsec\");\n}\n";
        let r = detect("src/handlers/webhook/stripe.rs", src);
        assert!(
            r.iter()
                .any(|v| v.pattern == "parse-before-verify" && v.severity == WhSeverity::P0Block)
        );
    }

    #[test]
    fn correct_order_ok() {
        let src = "use stripe;\nasync fn handle_webhook(body: &[u8], sig: &str) {\n    stripe::Webhook::construct_event(body, sig, \"whsec\");\n    let _evt: serde_json::Value = serde_json::from_slice(body).unwrap();\n}\n";
        let r = detect("src/handlers/webhook/stripe.rs", src);
        assert!(!r.iter().any(|v| v.pattern == "parse-before-verify"));
    }

    #[test]
    fn no_replay_window_advisory() {
        let src = "async fn handle_webhook() {\n    let _h = \"X-Hub-Signature\";\n    let _ = verify_signature();\n}\nfn verify_signature() {}\n";
        let r = detect("src/handlers/webhook/github.rs", src);
        assert!(r.iter().any(|v| v.pattern == "no-replay-window"));
    }

    #[test]
    fn replay_window_present_ok() {
        let src = "async fn handle_webhook() {\n    let _h = \"X-Hub-Signature\";\n    let _ts = \"timestamp\";\n    let _nonce = \"nonce\";\n}\n";
        let r = detect("src/handlers/webhook/github.rs", src);
        assert!(!r.iter().any(|v| v.pattern == "no-replay-window"));
    }

    #[test]
    fn non_webhook_file_skipped() {
        let src = "let evt: serde_json::Value = serde_json::from_slice(body).unwrap(); verify_signature();";
        let r = detect("src/handlers/users.rs", src);
        assert!(r.is_empty());
    }

    #[test]
    fn test_file_skipped() {
        let src = "use stripe;\nasync fn h(body: &[u8]) { let _: serde_json::Value = serde_json::from_slice(body).unwrap(); stripe::Webhook::construct_event(body, \"\", \"\"); }\n";
        let r = detect("crate/tests/webhook_stripe.rs", src);
        assert!(r.is_empty());
    }
}

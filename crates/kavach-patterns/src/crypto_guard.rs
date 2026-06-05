use regex::Regex;
use std::fmt::Write;
use std::sync::LazyLock;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CryptoFinding {
    pub banned: String,
    pub replacement: &'static str,
    pub line: usize,
}

struct Rule {
    re: Regex,
    replacement: &'static str,
}

fn mk(pat: &str, repl: &'static str) -> Option<Rule> {
    Regex::new(pat).map_or_else(
        |_| None,
        |re| {
            Some(Rule {
                re,
                replacement: repl,
            })
        },
    )
}

/// RFC-mandated exemption contexts — never flag.
/// REMOVED: ecdsa-p256-sha256 — RFC 9421 supports Ed25519, use `ed25519_dalek`.
static EXEMPTIONS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec![
        "interact_hash".into(),
        ["HMAC-SHA", "1"].concat(),
        ["HMAC-SHA", "256"].concat(),
        "stripe".into(),
        "paypal".into(),
        "resend".into(),
        "migration_033".into(),
    ]
});

static RULES: LazyLock<Vec<Rule>> = LazyLock::new(build);

fn build() -> Vec<Rule> {
    let hash_re = [
        "(?i)\\b(sha",
        "256|sha",
        "-256|sha",
        "2::|Sha",
        "256|sha",
        "1|md5|RIPEMD)\\b",
    ]
    .concat();
    let kdf_re = ["(?i)\\b(hk", "df|HK", "DF|pbk", "df2|bcr", "ypt)\\b"].concat();
    let aead_re = [
        "(?i)\\b(AES-256-G",
        "CM|Aes256G",
        "cm|aes_g",
        "cm|aes-g",
        "cm)\\b",
    ]
    .concat();
    let asym_re = ["(?i)\\b(EC", "DH|P-2", "56|secp256", "k1|RSA|ECD", "SA)\\b"].concat();
    let jwt_re = ["(?i)\\b(json", "webtoken|JW", "T|JW", "E|jo", "se)\\b"].concat();
    let pasetors_re = ["(?i)\\bpas", "etors\\b"].concat();

    vec![
        mk(&hash_re, "blake3::hash()"),
        mk(&kdf_re, "blake3::derive_key(\"ctx-v1\", ikm)"),
        mk(&aead_re, "XChaCha20Poly1305 (192-bit nonce, OsRng)"),
        mk(&asym_re, "Ed25519 (sign) + X25519 (key exchange)"),
        mk(&jwt_re, "PASETO v4 via IronGate only"),
        mk(&pasetors_re, "PASETO via IronGate — pasetors banned"),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn is_exempt(line: &str) -> bool {
    let lower = line.to_lowercase();
    EXEMPTIONS.iter().any(|e| lower.contains(&e.to_lowercase()))
}

/// Scan content for banned crypto usage.
#[must_use]
pub fn detect(file_path: &str, content: &str) -> Vec<CryptoFinding> {
    if content.is_empty() || crate::file_types::is_test_file(file_path) {
        return vec![];
    }
    // Skip pattern-detection files
    if file_path.contains("kavach-patterns/src/") {
        return vec![];
    }
    // Skip IronGate auth crate — it is the LEGITIMATE crypto owner per project
    // rule IRONGATE_SEPARATION ("IronGate OWNS all crypto — PASETO, tokens, key
    // management"). pasetors + AES-GCM (XChaCha20Poly1305 wrapper) usage inside
    // crates/services/irongate/ is policy-allowed; only Backend (core-auth,
    // platform crates, etc.) should remain crypto-free.
    if file_path.contains("crates/services/irongate/") {
        return vec![];
    }

    let rules = &*RULES;
    let mut findings = Vec::new();

    for (i, line) in content.lines().enumerate() {
        if is_exempt(line) {
            continue;
        }
        for rule in rules {
            if let Some(m) = rule.re.find(line) {
                findings.push(CryptoFinding {
                    banned: m.as_str().to_owned(),
                    replacement: rule.replacement,
                    line: i.saturating_add(1),
                });
            }
        }
    }
    findings
}

/// Block message if banned crypto found.
#[must_use]
pub fn check(file_path: &str, content: &str) -> Option<String> {
    let findings = detect(file_path, content);
    if findings.is_empty() {
        return None;
    }

    let mut msg = String::from("BOUNTY_CRYPTO_BLOCK:\n");
    for f in &findings {
        writeln!(
            &mut msg,
            "  BANNED: '{}' at L{}. USE: {}",
            f.banned, f.line, f.replacement
        )
        .ok();
    }
    msg.push_str("\nRESEARCH: WebSearch \"modern cryptography best practices {search_year}\"\n");
    msg.push_str("SKILL: Invoke `rust` skill (security section) for approved crypto stack.\n");
    msg.push_str("STACK: BLAKE3 (hash), XChaCha20Poly1305 (AEAD), Ed25519 (sign), X25519 (KEX), PASETO v4 (tokens)\n");
    Some(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_sha() {
        let code = ["use sha", "2::Sha", "256;"].concat();
        let f = detect("src/auth.rs", &code);
        assert!(!f.is_empty());
        assert!(f.first().is_some_and(|x| x.replacement.contains("blake3")));
    }

    #[test]
    fn blocks_aes() {
        let code = ["use aes", "_gcm::Aes", "256Gcm;"].concat();
        let f = detect("src/crypto.rs", &code);
        assert!(!f.is_empty());
        assert!(
            f.first()
                .is_some_and(|x| x.replacement.contains("XChaCha20"))
        );
    }

    #[test]
    fn blocks_jwt_crate() {
        let code = ["use json", "webtoken::encode;"].concat();
        let f = detect("src/auth.rs", &code);
        assert!(!f.is_empty());
        assert!(f.first().is_some_and(|x| x.replacement.contains("PASETO")));
    }

    #[test]
    fn allows_gnap_exemption() {
        let code = ["let h = sha", "256_interact_hash(&nonce);"].concat();
        assert!(detect("src/gnap.rs", &code).is_empty());
    }

    #[test]
    fn allows_totp_exemption() {
        let code = ["let mac = HMAC-SHA", "256::new(&key);"].concat();
        assert!(detect("src/totp.rs", &code).is_empty());
    }

    #[test]
    fn allows_stripe_exemption() {
        let code = ["let sig = stripe::verify_sha", "256(&payload);"].concat();
        assert!(detect("src/webhooks.rs", &code).is_empty());
    }

    #[test]
    fn skips_test_files() {
        let code = ["use sha", "2::Sha", "256;"].concat();
        assert!(detect("src/tests/crypto.rs", &code).is_empty());
    }

    #[test]
    fn skips_patterns_crate() {
        let code = ["use sha", "2::Sha", "256;"].concat();
        assert!(detect("kavach-patterns/src/guard.rs", &code).is_empty());
    }

    #[test]
    fn allows_irongate_pasetors_per_separation_rule() {
        // IronGate is the LEGITIMATE PASETO owner per project rule
        // IRONGATE_SEPARATION ("IronGate OWNS all crypto"). Backend remains
        // pasetors-free; only IronGate writes new pasetors:: code.
        let code = ["use pas", "etors::keys::Symme", "tricKey;"].concat();
        assert!(detect("crates/services/irongate/src/paseto_transfer_key.rs", &code).is_empty());
        assert!(detect("crates/services/irongate/src/grpc.rs", &code).is_empty());
    }

    #[test]
    fn still_blocks_pasetors_in_backend_core_auth() {
        // The exemption is path-scoped — Backend core-auth must NOT be allowed
        // to introduce pasetors:: usage (that's the violation we're enforcing).
        let code = ["use pas", "etors::keys::Symme", "tricKey;"].concat();
        let f = detect("crates/core/auth/src/handlers/session_transfer.rs", &code);
        assert!(
            !f.is_empty(),
            "core-auth must still trigger BOUNTY_CRYPTO_BLOCK"
        );
    }

    #[test]
    fn check_returns_block() {
        let code = [
            "use sha",
            "2::Sha",
            "256;\nlet h = Sha",
            "256::digest(data);",
        ]
        .concat();
        assert!(check("src/auth.rs", &code).is_some());
    }

    #[test]
    fn blocks_ecdsa_no_longer_exempt() {
        // ecdsa-p256-sha256 exemption removed — RFC 9421 supports Ed25519
        let code = ["use p256::ec", "dsa::SigningKey;"].concat();
        let f = detect("src/httpsig.rs", &code);
        assert!(!f.is_empty(), "ECDSA should be blocked, use Ed25519");
        assert!(f.first().is_some_and(|x| x.replacement.contains("Ed25519")));
    }
}

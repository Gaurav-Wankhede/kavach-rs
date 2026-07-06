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
    // IronGate is the LEGITIMATE PASETO authority per project rule
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
        "core-auth must still trigger [CRYPTO_SAFETY]"
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

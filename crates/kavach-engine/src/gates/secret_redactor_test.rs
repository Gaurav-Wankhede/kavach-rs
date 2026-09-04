//! Unit tests for secret and environment redaction.
use super::{is_env_path, redact_env_text, redact_secrets, sanitize_output};

#[test]
fn test_is_env_path() {
    assert!(is_env_path(".env"));
    assert!(is_env_path(".env.local"));
    assert!(is_env_path(".env.production"));
    assert!(is_env_path("/path/to/project/.env"));
    assert!(!is_env_path("main.rs"));
    assert!(!is_env_path("package.json"));
}

#[test]
fn test_redact_secrets_high_entropy() {
    let aws = "Found key AKIAIOSFODNN7EXAMPLE in logs";
    let (redacted, count) = redact_secrets(aws);
    assert_eq!(count, 1);
    assert!(redacted.contains("[REDACTED: AWS Access Key]"));
    assert!(!redacted.contains("AKIAIOSFODNN7EXAMPLE"));

    let gh = "Token is ghp_123456789012345678901234567890123456";
    let (redacted_gh, count_gh) = redact_secrets(gh);
    assert_eq!(count_gh, 1);
    assert!(redacted_gh.contains("[REDACTED: GitHub Token]"));
    assert!(!redacted_gh.contains("ghp_123456789012345678901234567890123456"));
}

#[test]
fn test_redact_env_text_preserves_keys() {
    let env_content = r#"
# Database Configuration
PORT=3000
NODE_ENV=production
DATABASE_URL="postgres://user:super_secret_pw@localhost:5432/mydb"
STRIPE_API_KEY=sk_live_1234567890abcdefghijklmnopqrstuvwxyz
PRIVATE_SIGNING_KEY="MIICXAIBAAKCAQEA0"
"#;

    let (redacted, count) = redact_env_text(env_content);
    assert_eq!(count, 3);
    // Keys preserved
    assert!(redacted.contains("PORT=3000"));
    assert!(redacted.contains("NODE_ENV=production"));
    assert!(redacted.contains("DATABASE_URL="));
    assert!(redacted.contains("STRIPE_API_KEY="));
    assert!(redacted.contains("PRIVATE_SIGNING_KEY="));

    // Secret values redacted
    assert!(!redacted.contains("super_secret_pw"));
    assert!(!redacted.contains("sk_live_1234567890abcdefghijklmnopqrstuvwxyz"));
    assert!(!redacted.contains("MIICXAIBAAKCAQEA0"));
    assert!(redacted.contains("postgres://..."));
    assert!(redacted.contains("sk_live_..."));
}

#[test]
fn test_sanitize_output_combines_passes() {
    let raw = "DATABASE_URL=postgres://root:pwd@localhost:5432\nAPI_KEY=AKIAIOSFODNN7EXAMPLE";
    let sanitized = sanitize_output(raw, Some(".env"));
    assert!(sanitized.contains("DATABASE_URL="));
    assert!(!sanitized.contains("pwd"));
    assert!(sanitized.contains("[REDACTED:"));
}

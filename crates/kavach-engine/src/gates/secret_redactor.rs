//! Secret & Environment Redaction Layer (info-leak closed).
use std::sync::OnceLock;
use regex::Regex;

#[cfg(test)]
#[path = "secret_redactor_test.rs"]
mod tests;

/// Redaction patterns and labels for high-entropy secrets.
struct SecretRule {
    regex: &'static str,
    label: &'static str,
}

static SECRET_RULES: &[SecretRule] = &[
    SecretRule {
        regex: r"-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z0-9 ]*PRIVATE KEY-----",
        label: "[REDACTED: Private Key]",
    },
    SecretRule {
        regex: r"\b(AKIA|ASIA|ABIA|ACCA)[0-9A-Z]{16}\b",
        label: "[REDACTED: AWS Access Key]",
    },
    SecretRule {
        regex: r"\b(ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9_]{36,255}\b",
        label: "[REDACTED: GitHub Token]",
    },
    SecretRule {
        regex: r"\bsk-ant-api03-[A-Za-z0-9\-_]{80,120}\b",
        label: "[REDACTED: Anthropic API Key]",
    },
    SecretRule {
        regex: r"\bsk-(proj-)?[A-Za-z0-9_-]{32,100}\b",
        label: "[REDACTED: OpenAI API Key]",
    },
    SecretRule {
        regex: r"\b(sk|rk)_(live|test)_[0-9a-zA-Z]{24,99}\b",
        label: "[REDACTED: Stripe Key]",
    },
    SecretRule {
        regex: r"\bxox[baprs]-[A-Za-z0-9-]{10,72}\b",
        label: "[REDACTED: Slack Token]",
    },
    SecretRule {
        regex: r"\beyJ[A-Za-z0-9-_=]{10,}\.[A-Za-z0-9-_=]{10,}\.[A-Za-z0-9-_=]{10,}\b",
        label: "[REDACTED: JWT Token]",
    },
];

static COMPILED_RULES: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();

fn compiled_rules() -> &'static [(Regex, &'static str)] {
    COMPILED_RULES.get_or_init(|| {
        SECRET_RULES
            .iter()
            .filter_map(|r| Regex::new(r.regex).ok().map(|re| (re, r.label)))
            .collect()
    })
}

/// Redact high-entropy secrets across arbitrary text.
#[must_use]
pub fn redact_secrets(text: &str) -> (String, usize) {
    let mut current = text.to_owned();
    let mut total: usize = 0;

    for (re, label) in compiled_rules() {
        let matches = re.find_iter(&current).count();
        if matches > 0 {
            total = total.saturating_add(matches);
            current = re.replace_all(&current, *label).to_string();
        }
    }

    (current, total)
}

/// Check whether a key is safe to display raw without masking.
fn is_safe_config_key(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    let is_standard = matches!(
        upper.as_str(),
        "PORT" | "HOST" | "NODE_ENV" | "STAGE" | "APP_ENV" | "DEBUG" | "LOG_LEVEL" | "RUST_LOG" | "ENVIRONMENT" | "TZ"
    );
    let has_sensitive_word = upper.contains("SECRET")
        || upper.contains("KEY")
        || upper.contains("TOKEN")
        || upper.contains("PASS")
        || upper.contains("AUTH")
        || upper.contains("CREDENTIAL")
        || upper.contains("PRIVATE")
        || upper.contains("URL")
        || upper.contains("DSN")
        || upper.contains("CERT");

    is_standard && !has_sensitive_word
}

/// Format a masked representation of an env value preserving useful hints.
fn mask_env_value(val: &str) -> String {
    let trimmed = val.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let (quote, inner) = if (trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2)
        || (trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2)
    {
        let q = trimmed.chars().next().unwrap_or('"');
        let slice = trimmed.get(1..trimmed.len().saturating_sub(1)).unwrap_or("");
        (Some(q), slice)
    } else {
        (None, trimmed)
    };

    let len = inner.len();
    let masked = inner.find("://").map_or_else(
        || {
            if inner.starts_with("sk_live_") || inner.starts_with("sk_test_") || inner.starts_with("ghp_") {
                let prefix = inner.get(..8).unwrap_or("");
                format!("[REDACTED: {prefix}... {len} chars]")
            } else {
                format!("[REDACTED: {len} chars]")
            }
        },
        |idx| {
            let scheme = inner.get(..idx.saturating_add(3)).unwrap_or("");
            format!("[REDACTED: {scheme}... {len} chars]")
        },
    );

    quote.map_or_else(|| masked.clone(), |q| format!("{q}{masked}{q}"))
}

/// Redact `.env` format text key-preserving: variable names kept, values masked.
#[must_use]
pub fn redact_env_text(text: &str) -> (String, usize) {
    let mut out = Vec::new();
    let mut redacted_count: usize = 0;

    for line in text.lines() {
        let trimmed_start = line.trim_start();
        if trimmed_start.starts_with('#') || trimmed_start.is_empty() {
            out.push(line.to_owned());
            continue;
        }

        let without_export = trimmed_start.strip_prefix("export ").unwrap_or(trimmed_start);
        if let Some(eq_idx) = without_export.find('=') {
            let key = without_export.get(..eq_idx).unwrap_or("").trim();
            let val = without_export.get(eq_idx.saturating_add(1)..).unwrap_or("");

            if is_safe_config_key(key) {
                out.push(line.to_owned());
            } else {
                redacted_count = redacted_count.saturating_add(1);
                let masked = mask_env_value(val);
                let prefix = if trimmed_start.starts_with("export ") { "export " } else { "" };
                let indent = line.get(..line.len().saturating_sub(trimmed_start.len())).unwrap_or("");
                out.push(format!("{indent}{prefix}{key}={masked}"));
            }
        } else {
            out.push(line.to_owned());
        }
    }

    (out.join("\n"), redacted_count)
}

/// Check if file path represents an environment configuration file.
#[must_use]
pub fn is_env_path(path: &str) -> bool {
    let filename = std::path::Path::new(path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(path);
    filename == ".env" || filename.starts_with(".env.")
}

/// Sanitize any tool response or file content before injecting into LLM context.
#[must_use]
pub fn sanitize_output(text: &str, file_hint: Option<&str>) -> String {
    let should_env_redact = file_hint.map_or_else(
        || {
            let env_lines = text
                .lines()
                .filter(|l| {
                    let t = l.trim();
                    !t.is_empty() && !t.starts_with('#') && t.contains('=')
                })
                .count();
            let total_nonempty = text.lines().filter(|l| !l.trim().is_empty()).count();
            env_lines >= 3 && env_lines.saturating_mul(2) >= total_nonempty
        },
        is_env_path,
    );

    let (first_pass, _) = if should_env_redact {
        redact_env_text(text)
    } else {
        (text.to_owned(), 0)
    };

    let (second_pass, _) = redact_secrets(&first_pass);
    second_pass
}

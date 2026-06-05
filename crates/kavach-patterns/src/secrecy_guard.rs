// split: intentional — guard module, not handler
//! Secrecy guard — detect sensitive values in plain String instead of Secret<T>.

use regex::Regex;
use std::sync::LazyLock;

struct SecrecyPattern {
    re: Regex,
    fix: &'static str,
}

fn mk(pat: &str, fix: &'static str) -> Option<SecrecyPattern> {
    Regex::new(pat).map_or_else(|_| None, |re| Some(SecrecyPattern { re, fix }))
}

static PATTERNS: LazyLock<Vec<SecrecyPattern>> = LazyLock::new(|| {
    let kw = [
        "tok",
        "en|api",
        "_key|sec",
        "ret|pass",
        "word|priv",
        "ate_key|cred",
        "entials",
    ]
    .concat();
    let re1 = format!("(?i)let\\s+({kw})\\s*:\\s*String");
    let re2 = format!("(?i)({kw})\\s*:\\s*String");
    vec![
        mk(&re1, "Use secrecy::Secret<String> for sensitive fields"),
        mk(
            &re2,
            "Use secrecy::Secret<String> for sensitive struct fields",
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
});

/// Advisory message if plain String used for sensitive values.
pub fn advise(file_path: &str, content: &str) -> Option<String> {
    if content.is_empty() || crate::is_test_file(file_path) {
        return None;
    }
    if !crate::is_code_file(file_path) {
        return None;
    }

    let mut findings = Vec::new();
    for (i, line) in content.lines().enumerate() {
        for p in PATTERNS.iter() {
            if p.re.is_match(line) {
                findings.push(format!(
                    "  L{}: {} — {}",
                    i.saturating_add(1),
                    line.trim(),
                    p.fix
                ));
            }
        }
    }
    if findings.is_empty() {
        return None;
    }
    let mut msg = String::from("[SECRECY_ADVISORY]\n");
    for f in &findings {
        msg.push_str(f);
        msg.push('\n');
    }
    Some(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_plain_string_token() {
        let code = ["let to", "ken: String = get_to", "ken();"].concat();
        assert!(advise("src/auth.rs", &code).is_some());
    }

    #[test]
    fn allows_normal_string() {
        assert!(advise("src/main.rs", "let name: String = get_name();").is_none());
    }

    #[test]
    fn skips_tests() {
        let code = ["let to", "ken: String = \"test\";"].concat();
        assert!(advise("src/tests/t.rs", &code).is_none());
    }
}

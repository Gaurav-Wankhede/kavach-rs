use regex::Regex;
use std::sync::LazyLock;

struct Pattern {
    re: Regex,
    name: String,
}

fn mk(pat: &str, name: &str) -> Option<Pattern> {
    Regex::new(pat).ok().map(|re| Pattern {
        re,
        name: name.into(),
    })
}

static PATTERNS: LazyLock<Vec<Pattern>> = LazyLock::new(build);

fn build() -> Vec<Pattern> {
    let pkey_re = ["-----BEGIN ", "(RSA |EC |DSA )?PRIV", "ATE KEY-----"].concat();
    let pkey_nm = ["Priv", "ate Key"].concat();
    let db_re = ["(?i)(post", "gres|my", "sql|mon", "godb)://\\S+:\\S+@"].concat();
    let db_nm = ["DB URL with cred", "entials"].concat();
    let api_re = ["(?i)(api[_\\-]?", "key)\\s*[:=]\\s*\\S{8,}"].concat();

    let defs = vec![
        (r"AKIA\w{16}", "AWS Access Key"),
        (r"gh[ps]_\w{36,}", "GitHub Token"),
        (&api_re, "API Key"),
        (r"eyJ\w{10,}\.\w{10,}\.\w{10,}", "JWT Token"),
        (&pkey_re, &pkey_nm),
        (r"xox[bpas]-\S+", "Slack Token"),
        (&db_re, &db_nm),
    ];

    defs.into_iter().filter_map(|(p, n)| mk(p, n)).collect()
}

/// Check content against bounty secret patterns.
/// Returns the pattern name if a match is found.
pub fn check(content: &str) -> Option<String> {
    PATTERNS
        .iter()
        .find(|p| p.re.is_match(content))
        .map(|p| p.name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_aws_key() {
        assert!(check("let k = \"AKIAIOSFODNN7EXAMPLE\";").is_some());
    }

    #[test]
    fn detects_github_token() {
        let tok = ["ghp_ABCDEF", "ghijklmnopqrstuvwxyz0123456789ab"].concat();
        assert!(check(&tok).is_some());
    }

    #[test]
    fn detects_jwt() {
        let jwt = [
            "eyJhbGciOiJIUz",
            "I1NiJ9.eyJzdWIi",
            "OiIxMjM0NTY3OD",
            "kwIiwiZXhwIjoxN",
            "jE2MjM5MDIyfQ.",
            "SflKxwRJSMeKKF2",
            "QT4fwpMeJf36POk",
            "6yJV_adQssw5c",
        ]
        .concat();
        assert!(check(&jwt).is_some());
    }

    #[test]
    fn detects_slack_token() {
        // Use split token to avoid GitHub secret scanning false positive
        let token = ["xoxb-", "123456789-", "abcdefghijklmnop"].concat();
        assert!(check(&token).is_some());
    }

    #[test]
    fn normal_code_passes() {
        assert!(check("fn main() { let x = 42; }").is_none());
    }
}

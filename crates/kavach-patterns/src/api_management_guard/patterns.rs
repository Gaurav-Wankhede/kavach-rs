use regex::Regex;
use std::sync::LazyLock;

pub(super) static PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    let stripe_live = format!("{}_{}", "sk", "live");
    let stripe_pub = format!("{}_{}", "pk", "live");
    let aws_access = String::from("AKIA");
    let github_pat = format!("{}{}", "gh", "p_");
    let github_pat2 = format!("{}_{}", "github", "pat_");
    let slack_bot = format!("{}{}-", "xo", "xb");
    let google_api = String::from("AIza");
    let cred_pat = format!(
        r#"[`'"](?:{stripe_live}|{stripe_pub}|{aws_access}|{github_pat}|{github_pat2}|{slack_bot}|{google_api})[A-Za-z0-9_-]{{10,}}[`'"]"#
    );
    let patterns = vec![
        Regex::new(r#"\bfetch\s*\(\s*[`'"]"#).ok(),
        Regex::new(r#"[`'"]https?://(?:api|backend|server)\.[a-z0-9.\-/]+[`'"]"#).ok(),
        Regex::new(r#"\.(?:route|get|post|put|patch|delete)\s*\(\s*[`'"]\s*/[a-zA-Z_]"#).ok(),
        Regex::new(r"async\s+fn\s+(?:list|get_all|fetch_all|index)_\w+\s*\([^)]*\)").ok(),
        Regex::new(r"Json<serde_json::Value>|Json<Value>|web::Json<serde_json::Value>").ok(),
        Regex::new(r"->\s*Result<Json<(?:User|Account|Order|Customer|Payment|Card)>").ok(),
        Regex::new(r"pub\s+async\s+fn\s+\w+\s*\([^)]*\)\s*->\s*(?:Result<)?Json<").ok(),
        Regex::new(r#"(?:Access-Control-Allow-Origin|allow_origin)[^=]*[=:]\s*[`'"]\s*\*\s*[`'"]"#)
            .ok(),
        Regex::new(r"\.fetch\s*\(|new\s+Request\s*\(").ok(),
        Regex::new(r"async\s+fn\s+\w*(?:webhook|webhook_handler)\w*\s*\(").ok(),
        Regex::new(r"Stripe-Signature|X-Hub-Signature|X-(?:Slack|GitHub|Twilio)-Signature").ok(),
        Regex::new(r#"(?:query|fetch_all|fetch_one|fetch_optional)\s*\(\s*[`'"][^`'"]+\bWHERE\b"#)
            .ok(),
        #[expect(
            clippy::trivial_regex,
            reason = "literal string match required for pattern consistency"
        )]
        Regex::new(r"Router::new\(\)").ok(),
        Regex::new(r"impl\s+IntoResponse\s+for\s+\w*Error").ok(),
        Regex::new(r"struct\s+\w*Response\s*\{[^}]*\bid:\s*(?:i32|i64|u32|u64)\b").ok(),
        Regex::new(r#"\.(?:post|put|patch)\s*\(\s*[`'"][^`'"]+[`'"]"#).ok(),
        Regex::new(r"BEGIN\s*;|pool\.begin\(\)").ok(),
        Regex::new(r"(?:stripe|sendgrid|twilio|slack|aws_sdk_\w+)::Client::new\s*\(").ok(),
        Regex::new(&cred_pat).ok(),
        Regex::new(r"(?:jsonwebtoken::decode|jwt::decode|paseto::decrypt)").ok(),
    ];
    patterns.into_iter().flatten().collect()
});

pub(super) fn is_api_relevant(path: &str) -> bool {
    let p = path.to_lowercase();
    super::boundary::has_extension(&p, ".rs")
        || super::boundary::has_extension(&p, ".ts")
        || super::boundary::has_extension(&p, ".tsx")
        || super::boundary::has_extension(&p, ".js")
        || super::boundary::has_extension(&p, ".jsx")
        || super::boundary::has_extension(&p, ".vue")
        || super::boundary::has_extension(&p, ".svelte")
        || super::boundary::has_extension(&p, ".astro")
        || super::boundary::has_extension(&p, ".go")
        || super::boundary::has_extension(&p, ".py")
}

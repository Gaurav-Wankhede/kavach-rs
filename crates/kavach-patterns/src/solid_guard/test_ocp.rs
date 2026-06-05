use crate::solid_guard::detect;

#[test]
fn ocp_provider_match_flagged() {
    let src = r"
use sqlx;
async fn x() {}
fn route(p: Provider) { match p { Provider::Stripe => 1, Provider::Paypal => 2, _ => 0 }; }
";
    let r = detect("src/services/payment.rs", src);
    assert!(r.iter().any(|v| v.pattern == "ocp-provider-match"));
}

#[test]
fn ocp_string_dispatch_flagged() {
    let src = r#"
use sqlx;
async fn x() {}
fn route(p: &str) { if p == "stripe" { } }
"#;
    let r = detect("src/services/pay.rs", src);
    assert!(r.iter().any(|v| v.pattern == "ocp-string-dispatch"));
}

#[test]
fn ocp_policy_with_vendor_switch_flagged() {
    let src = r"
use sqlx;
async fn x() {}
fn process_payment(p: Provider) { match p { Provider::Stripe => {} _ => {} }; }
";
    let r = detect("src/services/pay.rs", src);
    assert!(
        r.iter()
            .any(|v| v.pattern == "ocp-policy-with-vendor-switch")
    );
}

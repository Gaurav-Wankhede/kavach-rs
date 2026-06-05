use crate::solid_guard::detect;

#[test]
fn dip_non_domain_imports_infra_ok() {
    let src = r"
use crate::infra::sqlx_user_repo::PgUserRepo;
async fn x() {}
pub fn foo() {}
";
    let r = detect("src/handlers/user.rs", src);
    assert!(!r.iter().any(|v| v.pattern == "dip-domain-imports-infra"));
}

#[test]
fn dip_lazy_global_client_flagged() {
    let src = r#"
use once_cell::sync::Lazy;
use sqlx;
async fn x() {}
static POOL: Lazy<sqlx::PgPool> = Lazy::new(|| panic!("init"));
"#;
    let r = detect("src/infra/global.rs", src);
    assert!(r.iter().any(|v| v.pattern == "dip-lazy-global-client"));
}

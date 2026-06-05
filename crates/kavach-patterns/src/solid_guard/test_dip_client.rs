use crate::solid_guard::detect;

#[test]
fn dip_concrete_client_param_flagged() {
    let src = r"
use sqlx;
async fn x() {}
pub async fn list_users(pool: &sqlx::PgPool) -> Vec<u64> { vec![] }
";
    let r = detect("src/handlers/users.rs", src);
    assert!(r.iter().any(|v| v.pattern == "dip-concrete-client-param"));
}

#[test]
fn dip_concrete_service_field_flagged() {
    let src = r"
use sqlx;
async fn x() {}
pub struct UserService {
    pool: sqlx::PgPool,
}
";
    let r = detect("src/services/user.rs", src);
    assert!(r.iter().any(|v| v.pattern == "dip-concrete-service-field"));
}

#[test]
fn axum_dip_extension_concrete_flagged() {
    let src = r"
use axum::Extension;
use sqlx;
pub async fn list(Extension(pool): Extension<sqlx::PgPool>) {}
";
    let r = detect("src/handlers/users.rs", src);
    assert!(r.iter().any(|v| v.pattern == "axum-dip-extension-concrete"));
}

#[test]
fn dip_domain_imports_infra_flagged() {
    let src = r"
use crate::infra::sqlx_user_repo::PgUserRepo;
async fn x() {}
pub fn foo() {}
";
    let r = detect("src/domain/user.rs", src);
    assert!(r.iter().any(|v| v.pattern == "dip-domain-imports-infra"));
}

use crate::solid_guard::{SolidLetter, detect};

#[test]
fn srp_god_struct_flagged() {
    let src = r"
use sqlx;
pub struct User {
    pub id: u64,
    pub email: String,
    pub name: String,
    pub phone: String,
    pub address: String,
    pub city: String,
    pub country: String,
    pub zip: String,
    pub avatar_url: String,
    pub bio: String,
}
async fn x() {}
";
    let r = detect("src/domain/user.rs", src);
    assert!(
        r.iter()
            .any(|v| v.pattern == "srp-god-struct" && v.letter == SolidLetter::S)
    );
}

#[test]
fn srp_long_async_fn_flagged() {
    let body: String = "    let _ = 1;\n".repeat(90);
    let src = ["use axum;\nasync fn handler() {\n", &body, "}\n"].concat();
    let r = detect("src/handlers/big.rs", &src);
    assert!(r.iter().any(|v| v.pattern == "srp-long-async-fn"));
}

#[test]
fn srp_conflated_derives_flagged() {
    let src = r"
use sqlx;
async fn x() {}
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct User { pub id: u64, pub email: String }
";
    let r = detect("src/domain/user.rs", src);
    assert!(r.iter().any(|v| v.pattern == "srp-conflated-derives"));
}

#[test]
fn srp_two_derives_ok() {
    let src = r"
use sqlx;
async fn x() {}
#[derive(Debug, Clone, Serialize)]
pub struct User { pub id: u64 }
";
    let r = detect("src/domain/user.rs", src);
    assert!(!r.iter().any(|v| v.pattern == "srp-conflated-derives"));
}

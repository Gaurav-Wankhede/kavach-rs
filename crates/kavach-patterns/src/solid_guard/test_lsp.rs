use crate::solid_guard::detect;

#[test]
fn lsp_panic_in_impl_flagged() {
    let src = r#"
use sqlx;
async fn x() {}
trait Bird { fn fly(&self); }
struct Penguin;
impl Bird for Penguin { fn fly(&self) { panic!("can't"); } }
"#;
    let r = detect("src/domain/birds.rs", src);
    assert!(r.iter().any(|v| v.pattern == "lsp-panic-in-trait-impl"));
}

#[test]
fn lsp_result_then_unwrap_flagged() {
    let src = r"
use sqlx;
async fn x() {}
fn load(id: u64) -> Result<User, Err> { fetch(id).unwrap() }
";
    let r = detect("src/repository/user.rs", src);
    assert!(r.iter().any(|v| v.pattern == "lsp-result-then-unwrap"));
}

#[test]
fn lsp_block_on_in_trait_impl_flagged() {
    let src = r"
use sqlx;
async fn x() {}
trait Repo { fn fetch(&self, id: u64) -> Vec<u8>; }
struct Mine;
impl Repo for Mine { fn fetch(&self, id: u64) -> Vec<u8> { tokio::runtime::Handle::block_on(async { vec![] }) } }
";
    let r = detect("src/repository/blockon.rs", src);
    assert!(r.iter().any(|v| v.pattern == "lsp-block-on-in-trait-impl"));
}

#[test]
fn axum_lsp_service_panic_flagged() {
    let src = r#"
use tower::Service;
async fn x() {}
struct MySvc;
impl<B> Service<http::Request<B>> for MySvc {
    type Response = http::Response<()>;
    type Error = ();
    type Future = std::future::Ready<Result<Self::Response, Self::Error>>;
    fn poll_ready(&mut self, _: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> { panic!("nope"); }
    fn call(&mut self, _: http::Request<B>) -> Self::Future { todo!() }
}
"#;
    let r = detect("src/services/middleware.rs", src);
    assert!(r.iter().any(|v| v.pattern == "axum-lsp-service-panic"));
}

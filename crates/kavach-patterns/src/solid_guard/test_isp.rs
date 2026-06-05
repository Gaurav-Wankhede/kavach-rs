use crate::solid_guard::detect;

#[test]
fn isp_fat_trait_flagged() {
    let src = r"
use sqlx;
async fn x() {}
pub trait Mega {
    fn a(&self);
    fn b(&self);
    fn c(&self);
    fn d(&self);
    fn e(&self);
    fn f(&self);
    fn g(&self);
    fn h(&self);
    fn i(&self);
}
";
    let r = detect("src/domain/mega.rs", src);
    assert!(r.iter().any(|v| v.pattern == "isp-fat-trait"));
}

#[test]
fn isp_storage_god_trait_flagged() {
    let src = r"
use sqlx;
async fn x() {}
pub trait Storage { fn get(&self); fn put(&self); fn delete(&self); }
";
    let r = detect("src/services/storage.rs", src);
    assert!(r.iter().any(|v| v.pattern == "isp-storage-god-trait"));
}

#[test]
fn isp_catchall_flagged() {
    let src = r"
use sqlx;
async fn x() {}
pub trait Worker { fn do_everything(&self); }
";
    let r = detect("src/services/worker.rs", src);
    assert!(r.iter().any(|v| v.pattern == "isp-catchall-method"));
}

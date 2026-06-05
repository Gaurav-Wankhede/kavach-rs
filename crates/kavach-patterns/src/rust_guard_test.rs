//! Test hub for `rust_guard` — declares the per-concern test leaves. Split out
//! of the former inline `mod tests` to honor the ≤100-LOC micro-file law; each
//! leaf is a child module so `crate::rust_guard::detect` + `RustSeverity` resolve.
#[path = "rust_guard/test_async_db.rs"]
mod async_db;
#[path = "rust_guard/test_p0_errors.rs"]
mod p0_errors;
#[path = "rust_guard/test_p0_structural.rs"]
mod p0_structural;
#[path = "rust_guard/test_p1_quality.rs"]
mod p1_quality;

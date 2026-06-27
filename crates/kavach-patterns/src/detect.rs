//! Anti-production pattern detection system.
//!
//! Detects: mock data, stubs, security issues, error handling gaps, type safety problems.
//! Organized into leaves per concern to maintain ≤100 LOC per file.
use crate::config::{AntiProdLevel, AntiProdResult};
use crate::file_types::is_allowlisted;
use crate::regex_patterns::P;
// Public leaf modules
mod errors_api;
mod errors_frontend;
mod errors_lang;
mod lang_docker;
mod lang_go;
mod lang_java;
mod lang_python;
mod lang_rust;
mod mock_data;
mod patterns_general;
mod patterns_handler;
mod security_frontend;
mod type_safety;
// Re-export public API
pub use mock_data::detect_mock_data;
/// Detect anti-production patterns in file content.
///
/// Returns an empty vec for allowlisted or empty files. Otherwise returns all violations found.
#[must_use]
pub fn detect_antiprod(fp: &str, content: &str) -> Vec<AntiProdResult> {
    if content.is_empty() || is_allowlisted(fp) {
        return vec![];
    }
    let (mut res, r) = (Vec::new(), &*P);
    if let Some(reason) = detect_mock_data(fp, content) {
        res.push(AntiProdResult {
            level: AntiProdLevel::P0MockData,
            code: "MOCK_DATA",
            match_text: reason,
            message: "Replace hardcoded data.",
        });
    }
    if crate::file_types::is_frontend_file(fp) {
        security_frontend::detect_frontend_security(&mut res, r, content);
    }
    patterns_general::detect_general_patterns(&mut res, r, fp, content);
    patterns_handler::detect_handler_patterns(&mut res, r, fp, content);
    if crate::file_types::is_rust_file(fp) {
        lang_rust::detect_rust_lang(&mut res, r, fp, content);
    }
    if crate::file_types::is_go_file(fp) {
        lang_go::detect_go_lang(&mut res, r, fp, content);
    }
    if crate::file_types::is_python_file(fp) {
        lang_python::detect_python_lang(&mut res, r, content);
    }
    if crate::file_types::is_java_file(fp) {
        lang_java::detect_java_lang(&mut res, r, content);
    }
    if crate::file_types::is_dockerfile(fp) {
        lang_docker::detect_dockerfile_lang(&mut res, r, content);
    }
    errors_frontend::detect_frontend_errors(&mut res, r, content);
    errors_api::detect_api_client_errors(&mut res, r, fp, content);
    errors_lang::detect_rust_errors(&mut res, r, fp, content);
    errors_lang::detect_go_errors(&mut res, r, fp, content);
    errors_lang::detect_python_errors(&mut res, r, content);
    errors_lang::detect_java_errors(&mut res, r, content);
    errors_lang::detect_docker_errors(&mut res, r, fp, content);
    type_safety::detect_type_safety(&mut res, r, fp, content);
    res
}
#[cfg(test)]
#[path = "detect_test.rs"]
#[path = "detect_test.rs"]
mod tests;

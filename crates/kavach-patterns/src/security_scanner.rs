//! Aggregated security scanner for DeepSec-style scanning.
//! Combines OWASP guard into a single scan pipeline.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SecurityFinding {
    pub file: String,
    pub line: usize,
    pub severity: String,
    pub category: String,
    pub pattern: String,
    pub fix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ScanResult {
    pub file: String,
    pub findings: Vec<SecurityFinding>,
}

/// Scan a single file for security vulnerabilities using OWASP guard.
#[must_use]
pub fn scan_file(file_path: &str, content: &str) -> ScanResult {
    let mut findings = Vec::new();

    // OWASP vulnerabilities (SQLi, XSS, SSRF, CMDi)
    for f in crate::owasp_guard::detect(file_path, content) {
        findings.push(SecurityFinding {
            file: file_path.to_owned(),
            line: f.line,
            severity: format!("{:?}", f.severity),
            category: f.category.to_owned(),
            pattern: f.pattern,
            fix: f.fix.to_owned(),
        });
    }

    ScanResult {
        file: file_path.to_owned(),
        findings,
    }
}

/// Security-sensitive file patterns for pre-filtering.
#[must_use]
pub fn is_security_sensitive(path: &Path) -> bool {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    // Skip non-code files
    let code_exts = ["rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "sql"];
    if !code_exts.contains(&ext) {
        return false;
    }

    // Skip test files for initial scan (can be included optionally)
    if crate::file_types::is_test_file(&path.to_string_lossy()) {
        return false;
    }

    // Prioritize security-relevant files
    let sensitive_patterns = [
        "auth",
        "login",
        "password",
        "credential",
        "token",
        "session",
        "crypto",
        "encrypt",
        "decrypt",
        "hash",
        "secret",
        "key",
        "api",
        "handler",
        "controller",
        "route",
        "endpoint",
        "database",
        "query",
        "sql",
        "migration",
        "upload",
        "download",
        "file",
        "storage",
        "admin",
        "permission",
        "role",
        "access",
        "payment",
        "billing",
        "transaction",
    ];

    let lower_name = name.to_lowercase();
    sensitive_patterns.iter().any(|p| lower_name.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_sql_injection() {
        let code = [
            "let q = fo",
            "rmat!(\"SE",
            "LE",
            "CT * FR",
            "OM users WH",
            "ERE id = {}\", uid);",
        ]
        .concat();
        let result = scan_file("src/handler.rs", &code);
        assert!(!result.findings.is_empty());
    }

    #[test]
    fn identifies_sensitive_files() {
        assert!(is_security_sensitive(Path::new("src/auth_handler.rs")));
        assert!(is_security_sensitive(Path::new("src/login.ts")));
        assert!(!is_security_sensitive(Path::new("src/utils.rs")));
        assert!(!is_security_sensitive(Path::new("README.md")));
    }
}

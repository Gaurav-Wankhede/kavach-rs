// split: Single-module gate file for Rust 1.96+ / Edition 2024 enforcement.
//! Rust 1.96+ / Edition 2024 Production Gate
//!
//! Dedicated guard for VERSION-SPECIFIC Rust patterns. Distinct from `rust_guard.rs`
//! (timeless anti-patterns). This file enforces Rust 1.96+ / Edition 2024 ONLY.
//!
//! SOURCES (verified 2026-05):
//! - <https://blog.rust-lang.org/2026/05/28/Rust-1.96.0>/
//! - <https://releases.rs/docs/1.96.0/>
//! - <https://blog.rust-lang.org/2026/04/16/Rust-1.95.0>/
//! - <https://doc.rust-lang.org/edition-guide/rust-2018/path-changes.html>
//! - <https://corrode.dev/blog/pitfalls-of-safe-rust>/
//! - <https://sherlock.xyz/post/rust-security-auditing-guide-2026>

use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "matched/constructed cross-crate; non_exhaustive => E0639/E0004"
)]
pub enum Rust196Severity {
    P0Block,
    P1Advisory,
    P2Warning,
}

#[derive(Debug, Clone)]
#[expect(
    clippy::exhaustive_structs,
    reason = "matched/constructed cross-crate; non_exhaustive => E0639/E0004"
)]
pub struct Rust196Violation {
    pub severity: Rust196Severity,
    pub pattern: &'static str,
    pub fix: &'static str,
    pub line: usize,
}

fn secret_field_regex() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        let priv_k = format!("{}_{}", "private", "key");
        let cred_k = format!("{}_{}", "api", "key");
        let sess_id = format!("{}_{}", "session", "id");
        let tokens = format!("password|secret|token|{cred_k}|{priv_k}|jwt|{sess_id}");
        Regex::new(&format!(
            r"#\[derive\([^)]*Debug[^)]*\)\][\s\S]{{0,200}}\b(?:{tokens})\b"
        ))
        .ok()
    })
    .as_ref()
}

fn get_pattern(idx: usize) -> Option<&'static Regex> {
    static P0: OnceLock<Option<Regex>> = OnceLock::new();
    static P1: OnceLock<Option<Regex>> = OnceLock::new();
    static P2: OnceLock<Option<Regex>> = OnceLock::new();
    static P3: OnceLock<Option<Regex>> = OnceLock::new();
    static P4: OnceLock<Option<Regex>> = OnceLock::new();
    static P5: OnceLock<Option<Regex>> = OnceLock::new();
    static P6: OnceLock<Option<Regex>> = OnceLock::new();
    static P7: OnceLock<Option<Regex>> = OnceLock::new();
    static P8: OnceLock<Option<Regex>> = OnceLock::new();
    static P9: OnceLock<Option<Regex>> = OnceLock::new();
    static P10: OnceLock<Option<Regex>> = OnceLock::new();
    static P11: OnceLock<Option<Regex>> = OnceLock::new();
    static P12: OnceLock<Option<Regex>> = OnceLock::new();
    static P13: OnceLock<Option<Regex>> = OnceLock::new();
    static P14: OnceLock<Option<Regex>> = OnceLock::new();
    static P15: OnceLock<Option<Regex>> = OnceLock::new();
    static P16: OnceLock<Option<Regex>> = OnceLock::new();
    static P17: OnceLock<Option<Regex>> = OnceLock::new();

    let init = |lock: &'static OnceLock<Option<Regex>>, pat: &str| {
        lock.get_or_init(|| Regex::new(pat).ok()).as_ref()
    };

    match idx {
        0 => init(&P0, r#"edition\s*=\s*"(?:2018|2021)""#),
        1 => init(&P1, r"cfg[-_]if\s*=\s*"),
        2 => init(&P2, r"async[-_]trait\s*=\s*"),
        3 => init(&P3, r"#\[async_trait\]"),
        4 => init(
            &P4,
            r"(?:price|quantity|total|amount|sum)\s*\*\s*(?:price|quantity|total|amount|count)",
        ),
        5 => init(
            &P5,
            r"Vec<i(?:8|16|32|64)>[\s\S]{0,100}(?://[^\n]*\b(?:indices|idx|index)\b|let\s+\w*ind)",
        ),
        6 => init(
            &P6,
            r"fn\s+\w*(?:read|write|open|load|save)_?\w*\([^)]*:\s*&str\)",
        ),
        7 => secret_field_regex(),
        8 => init(&P7, r"static\s+mut\s+\w+\s*:"),
        9 => init(
            &P8,
            r"async\s+fn\s+\w+[^{]*\{[\s\S]*?for\s+\w+\s+in\s+[^{]+\{[^}]*tokio::spawn",
        ),
        10 => init(
            &P9,
            r"if\s+let\s+[^=]+=[^{]+\{\s*if\s+let\s+[^=]+=[^{]+\{\s*if\s+let\s+[^=]+=[^{]+\{\s*if\s+let",
        ),
        11 => init(&P10, r"async\s+fn[\s\S]*?futures::executor::block_on"),
        12 => init(&P11, r"Box<dyn\s+Any\s*[+>]"),
        13 => init(&P12, r"\bcfg_if!\s*\{"),
        14 => init(
            &P13,
            r#"#\[cfg\(target_os\s*=\s*"\w+"\)\][\s\S]{0,80}#\[cfg\(target_os\s*=\s*"\w+"\)\]"#,
        ),
        15 => init(&P14, r"(?:cents|amount_cents|price_cents)\s*[+\-*]\s*\w"),
        16 => init(&P15, r#"std::env::var\("\w+"\)\.unwrap\(\)"#),
        // Rust 1.96: `use Path::{self as Name}` is rejected — `{self}` imports require a
        // module parent, so re-aliasing a struct/enum via {self as ..} no longer compiles.
        // SOURCE: https://releases.rs/docs/1.96.0/
        17 => init(&P16, r"use\s+[\w:]+::\{\s*self\s+as\s+\w+"),
        // Rust 1.96: when an item carries duplicate export_name / link_name / link_section,
        // the FIRST attribute now wins (was last). Two of the same on adjacent lines is a
        // silent behavior change worth surfacing.
        // SOURCE: https://blog.rust-lang.org/2026/05/28/Rust-1.96.0/
        18 => init(
            &P17,
            r"#\[(?:export_name|link_name|link_section)[\s\S]{0,120}#\[(?:export_name|link_name|link_section)",
        ),
        _ => None,
    }
}

#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "pattern table with 19 detection cases"
)]
pub fn detect(file_path: &str, content: &str) -> Vec<Rust196Violation> {
    let mut violations = Vec::new();

    if file_path.ends_with("/mod.rs") || file_path.ends_with("\\mod.rs") {
        violations.push(Rust196Violation {
            severity: Rust196Severity::P0Block,
            pattern: "legacy mod.rs file",
            fix: "Rust 2024 uses foo.rs + foo/ pattern. Rename mod.rs to parent_module_name.rs.",
            line: 0,
        });
    }

    if file_path.ends_with("Cargo.toml") {
        if get_pattern(0).is_some_and(|p| p.is_match(content)) {
            violations.push(Rust196Violation {
                severity: Rust196Severity::P0Block,
                pattern: "stale Rust edition",
                fix: "Edition 2024 required for let-chains, if-let guards, drop scoping. Set edition = \"2024\".",
                line: 0,
            });
        }
        if get_pattern(1).is_some_and(|p| p.is_match(content)) {
            violations.push(Rust196Violation {
                severity: Rust196Severity::P0Block,
                pattern: "cfg-if dependency",
                fix: "Rust 1.95+ stabilized cfg_select! macro. Remove cfg-if from Cargo.toml.",
                line: 0,
            });
        }
        if get_pattern(2).is_some_and(|p| p.is_match(content)) {
            violations.push(Rust196Violation {
                severity: Rust196Severity::P1Advisory,
                pattern: "async-trait dependency",
                fix: "Rust 1.75+ supports native async fn in traits (AFIT). Drop async-trait unless using dyn Trait.",
                line: 0,
            });
        }
        return violations;
    }

    if !std::path::Path::new(file_path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
    {
        return violations;
    }
    if crate::file_types::is_test_file(file_path) {
        return violations;
    }

    if get_pattern(3).is_some_and(|p| p.is_match(content)) {
        violations.push(Rust196Violation {
            severity: Rust196Severity::P0Block,
            pattern: "async_trait attribute",
            fix: "Use native async fn in trait (Rust 1.75+ AFIT). Remove the attribute unless needed for dyn Trait.",
            line: 0,
        });
    }
    if get_pattern(8).is_some_and(|p| p.is_match(content)) {
        violations.push(Rust196Violation {
            severity: Rust196Severity::P0Block,
            pattern: "static mut",
            fix: "Use AtomicU64/AtomicBool or Mutex<T>. static mut is unsound and lint-deny in 1.96.",
            line: 0,
        });
    }
    if get_pattern(11).is_some_and(|p| p.is_match(content)) {
        violations.push(Rust196Violation {
            severity: Rust196Severity::P0Block,
            pattern: "block_on in async fn",
            fix: "Never block_on inside async — use .await or tokio::task::spawn_blocking for sync work.",
            line: 0,
        });
    }
    if get_pattern(13).is_some_and(|p| p.is_match(content)) {
        violations.push(Rust196Violation {
            severity: Rust196Severity::P0Block,
            pattern: "cfg_if! macro",
            fix: "Rust 1.95+ has cfg_select! built-in. Replace cfg_if! and drop the cfg-if dep.",
            line: 0,
        });
    }
    if get_pattern(12).is_some_and(|p| p.is_match(content)) {
        violations.push(Rust196Violation {
            severity: Rust196Severity::P0Block,
            pattern: "Box<dyn Any>",
            fix: "Use generics or enum dispatch. Box<dyn Any> erases types — defeats Rust's type system.",
            line: 0,
        });
    }
    if get_pattern(7).is_some_and(|p| p.is_match(content)) {
        violations.push(Rust196Violation {
            severity: Rust196Severity::P0Block,
            pattern: "Debug on sensitive type",
            fix: "Use secrecy::Secret<T> or impl Debug manually with redaction. Auto Debug leaks confidential data.",
            line: 0,
        });
    }
    if get_pattern(16).is_some_and(|p| p.is_match(content)) {
        violations.push(Rust196Violation {
            severity: Rust196Severity::P0Block,
            pattern: "env::var unwrap at runtime",
            fix: "Validate ALL env vars at boot in main(). Runtime unwrap = production crash on missing config.",
            line: 0,
        });
    }
    if get_pattern(17).is_some_and(|p| p.is_match(content)) {
        violations.push(Rust196Violation {
            severity: Rust196Severity::P0Block,
            pattern: "{self as Name} struct/enum import",
            fix: "Rust 1.96 rejects `use Path::{self as Name}` for non-modules. Import the item directly: `use Path as Name`.",
            line: 0,
        });
    }
    if get_pattern(18).is_some_and(|p| p.is_match(content)) {
        violations.push(Rust196Violation {
            severity: Rust196Severity::P1Advisory,
            pattern: "duplicate export_name/link_name/link_section",
            fix: "Rust 1.96 flipped precedence to first-wins for these attrs. Keep one — duplicates silently changed which symbol/name applies.",
            line: 0,
        });
    }

    if get_pattern(10).is_some_and(|p| p.is_match(content)) {
        violations.push(Rust196Violation {
            severity: Rust196Severity::P1Advisory,
            pattern: "nested if let pyramid",
            fix: "Edition 2024 supports let chains: `if let A = a && let B = b { }`. Flatten the pyramid.",
            line: 0,
        });
    }
    if get_pattern(14).is_some_and(|p| p.is_match(content)) {
        violations.push(Rust196Violation {
            severity: Rust196Severity::P1Advisory,
            pattern: "manual cfg target_os pairs",
            fix: "Use cfg_select! { unix => {..}, windows => {..}, _ => {..} } for cross-platform code (Rust 1.95+).",
            line: 0,
        });
    }
    if get_pattern(9).is_some_and(|p| p.is_match(content)) {
        violations.push(Rust196Violation {
            severity: Rust196Severity::P1Advisory,
            pattern: "unbounded spawn in loop",
            fix: "Use tokio::sync::Semaphore + JoinSet to bound concurrency. Unbounded spawn = DoS via fanout.",
            line: 0,
        });
    }
    if get_pattern(5).is_some_and(|p| p.is_match(content)) {
        violations.push(Rust196Violation {
            severity: Rust196Severity::P1Advisory,
            pattern: "Vec<i32> for indices",
            fix: "Use Vec<usize>. Rust slice/Vec indexing requires usize.",
            line: 0,
        });
    }
    if get_pattern(6).is_some_and(|p| p.is_match(content)) {
        violations.push(Rust196Violation {
            severity: Rust196Severity::P1Advisory,
            pattern: "&str for filesystem path",
            fix: "Use &Path or impl AsRef<Path>. &str loses platform-specific path semantics.",
            line: 0,
        });
    }

    let has_safe_arith = content.contains("checked_") || content.contains("saturating_");
    if get_pattern(4).is_some_and(|p| p.is_match(content)) && !has_safe_arith {
        violations.push(Rust196Violation {
            severity: Rust196Severity::P2Warning,
            pattern: "unchecked arithmetic on money/qty",
            fix: "Use .checked_mul()/.checked_add() for currency. Release-mode overflow silently wraps.",
            line: 0,
        });
    }
    if get_pattern(15).is_some_and(|p| p.is_match(content)) && !has_safe_arith {
        violations.push(Rust196Violation {
            severity: Rust196Severity::P2Warning,
            pattern: "unchecked cents arithmetic",
            fix: "Money values must use checked_* — silent overflow corrupts financial records.",
            line: 0,
        });
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_legacy_mod_rs() {
        let v = detect("src/utils/mod.rs", "pub fn helper() {}");
        assert!(v.iter().any(|x| x.pattern == "legacy mod.rs file"));
    }

    #[test]
    fn allows_modern_module_layout() {
        let v = detect("src/utils.rs", "pub mod helpers;");
        assert!(!v.iter().any(|x| x.pattern == "legacy mod.rs file"));
    }

    #[test]
    fn detects_edition_2018() {
        let v = detect(
            "Cargo.toml",
            "[package]\nname = \"foo\"\nedition = \"2018\"",
        );
        assert!(v.iter().any(|x| x.pattern == "stale Rust edition"));
    }

    #[test]
    fn detects_edition_2021() {
        let v = detect("Cargo.toml", "[package]\nedition = \"2021\"");
        assert!(v.iter().any(|x| x.pattern == "stale Rust edition"));
    }

    #[test]
    fn allows_edition_2024() {
        let v = detect("Cargo.toml", "[package]\nedition = \"2024\"");
        assert!(!v.iter().any(|x| x.pattern == "stale Rust edition"));
    }

    #[test]
    fn detects_cfg_if_dependency() {
        let v = detect("Cargo.toml", "[dependencies]\ncfg-if = \"1.0\"");
        assert!(v.iter().any(|x| x.pattern == "cfg-if dependency"));
    }

    #[test]
    fn detects_async_trait_attribute() {
        let v = detect(
            "src/repo.rs",
            "#[async_trait]\npub trait Repo { async fn fetch(&self); }",
        );
        assert!(v.iter().any(|x| x.pattern == "async_trait attribute"));
    }

    #[test]
    fn detects_static_mut() {
        let v = detect("src/counter.rs", "static mut COUNTER: u32 = 0;");
        assert!(v.iter().any(|x| x.pattern == "static mut"));
    }

    #[test]
    fn detects_block_on_in_async() {
        let v = detect(
            "src/handler.rs",
            "async fn handler() { let x = futures::executor::block_on(fetch()); }",
        );
        assert!(v.iter().any(|x| x.pattern == "block_on in async fn"));
    }

    #[test]
    fn detects_cfg_if_macro() {
        let v = detect(
            "src/platform.rs",
            "cfg_if! { if #[cfg(unix)] { fn x() {} } else { fn x() {} } }",
        );
        assert!(v.iter().any(|x| x.pattern == "cfg_if! macro"));
    }

    #[test]
    fn detects_box_dyn_any() {
        let v = detect("src/store.rs", "fn store(v: Box<dyn Any + Send>) {}");
        assert!(v.iter().any(|x| x.pattern == "Box<dyn Any>"));
    }

    #[test]
    fn detects_debug_on_sensitive_type() {
        let v = detect(
            "src/user.rs",
            "#[derive(Debug, Clone)]\npub struct User { pub name: String, pub password: String }",
        );
        assert!(v.iter().any(|x| x.pattern == "Debug on sensitive type"));
    }

    #[test]
    fn detects_env_var_unwrap() {
        let v = detect(
            "src/config.rs",
            "let url = std::env::var(\"DATABASE_URL\").unwrap();",
        );
        assert!(v.iter().any(|x| x.pattern == "env::var unwrap at runtime"));
    }

    #[test]
    fn detects_self_as_struct_import() {
        let v = detect("src/lib.rs", "use crate::types::Widget::{self as W};");
        assert!(
            v.iter()
                .any(|x| x.pattern == "{self as Name} struct/enum import")
        );
    }

    #[test]
    fn allows_self_glob_module_import() {
        // `use foo::{self, bar}` on a real module stays legal — only `{self as ..}` is targeted.
        let v = detect("src/lib.rs", "use std::io::{self, Read};");
        assert!(
            !v.iter()
                .any(|x| x.pattern == "{self as Name} struct/enum import")
        );
    }

    #[test]
    fn detects_duplicate_link_attrs() {
        let v = detect(
            "src/ffi.rs",
            "#[link_name = \"foo\"]\n#[link_name = \"bar\"]\nextern \"C\" { fn f(); }",
        );
        assert!(
            v.iter()
                .any(|x| x.pattern == "duplicate export_name/link_name/link_section")
        );
    }

    #[test]
    fn detects_nested_if_let_pyramid() {
        let v = detect(
            "src/lib.rs",
            "if let Some(a) = x { if let Some(b) = y { if let Some(c) = z { if let Some(d) = w { } } } }",
        );
        assert!(v.iter().any(|x| x.pattern == "nested if let pyramid"));
    }

    #[test]
    fn detects_vec_i32_indices() {
        let v = detect(
            "src/lib.rs",
            "let mut indices: Vec<i32> = Vec::new(); // indices into the slice",
        );
        assert!(v.iter().any(|x| x.pattern == "Vec<i32> for indices"));
    }

    #[test]
    fn detects_str_for_path() {
        let v = detect(
            "src/config.rs",
            "pub fn read_config(path: &str) -> String { String::new() }",
        );
        assert!(v.iter().any(|x| x.pattern == "&str for filesystem path"));
    }

    #[test]
    fn allows_path_param() {
        let v = detect(
            "src/config.rs",
            "pub fn read_config(path: &Path) -> String { String::new() }",
        );
        assert!(!v.iter().any(|x| x.pattern == "&str for filesystem path"));
    }

    #[test]
    fn detects_unchecked_money_arithmetic() {
        let v = detect(
            "src/billing.rs",
            "let total_cents = price_cents + tax_cents;",
        );
        assert!(v.iter().any(|x| x.pattern == "unchecked cents arithmetic"));
    }

    #[test]
    fn allows_checked_money_arithmetic() {
        let v = detect(
            "src/billing.rs",
            "let total_cents = price_cents.checked_add(tax_cents).ok_or(Err)?;",
        );
        assert!(!v.iter().any(|x| x.pattern == "unchecked cents arithmetic"));
    }

    #[test]
    fn test_file_skipped() {
        let v = detect("/project/tests/integration.rs", "static mut TEST: u32 = 0;");
        assert!(v.is_empty());
    }
}

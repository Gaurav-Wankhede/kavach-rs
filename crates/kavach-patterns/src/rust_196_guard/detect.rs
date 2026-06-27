// split: cohesive 1.96 pattern-detector; the `async fn` tokens are fix-text strings, not handlers.
use super::patterns::get_pattern;
use super::types::{Rust196Severity, Rust196Violation};

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

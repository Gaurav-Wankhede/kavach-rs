// split: intentional — single guard module for design-pattern advisories
//! Rust Design Patterns Guard — coverage for the rust-unofficial catalog
//! gaps not detected by existing guards.
//!
//! SOURCES (verified 2026-05):
//! - <https://rust-unofficial.github.io/patterns>/
//! - <https://rust-unofficial.github.io/patterns/anti_patterns/deny-warnings.html>
//! - <https://rust-unofficial.github.io/patterns/anti_patterns/deref.html>
//! - <https://rust-unofficial.github.io/patterns/idioms/coercion-arguments.html>
//! - <https://rust-unofficial.github.io/patterns/patterns/creational/builder.html>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PatternSeverity {
    P1Advisory,
    P2Warning,
}
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PatternViolation {
    pub severity: PatternSeverity,
    pub pattern: &'static str,
    pub fix: &'static str,
    pub line: usize,
}
use crate::design_patterns_scan as scan;
/// Scan content for Rust design pattern advisories.
pub fn detect(file_path: &str, content: &str) -> Vec<PatternViolation> {
    if content.is_empty() || crate::file_types::is_test_file(file_path) {
        return vec![];
    }
    if !std::path::Path::new(file_path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("rs"))
    {
        return vec![];
    }
    let mut violations = Vec::with_capacity(4);
    for (i, line) in content.lines().enumerate() {
        for rule in crate::design_patterns_rules::RULES.iter() {
            if rule.re.as_ref().is_some_and(|re| re.is_match(line)) {
                violations.push(PatternViolation {
                    severity: rule.sev,
                    pattern: rule.pattern,
                    fix: rule.fix,
                    line: i.saturating_add(1),
                });
            }
        }
    }
    if let Some(line) = scan::many_arg_constructor(content) {
        violations.push(PatternViolation {
            severity: PatternSeverity::P1Advisory,
            pattern: "many-arg constructor without Builder",
            fix: "Constructor with >4 args is hard to call. Add Builder pattern.",
            line,
        });
    }
    if let Some(line) = scan::state_take_on_boxdyn(content) {
        violations.push(PatternViolation {
            severity: PatternSeverity::P1Advisory,
            pattern: "State transition: mem::take on Box<dyn> field",
            fix: "Box<dyn _> has no Default, so mem::take fails E0277. Model the slot as Option<Box<dyn _>> and use Option::take().",
            line,
        });
    }
    if let Some(line) = scan::flyweight_mut_ref(content) {
        violations.push(PatternViolation {
            severity: PatternSeverity::P1Advisory,
            pattern: "Flyweight: &mut self accessor returning &T",
            fix: "Returning &T from a &mut self method aliases the &mut borrow (E0499). Split into register(&mut self) then get(&self).",
            line,
        });
    }
    violations
}
#[cfg(test)]
#[path = "design_patterns_guard_test.rs"]
#[cfg(test)]
mod tests;

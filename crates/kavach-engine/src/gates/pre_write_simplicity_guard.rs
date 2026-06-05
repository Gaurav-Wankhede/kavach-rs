// Karpathy Principle 2: "Simplicity First"
// Advisory-only scanner for over-engineering signals in .rs files.
// P1 warnings only — never blocks.

/// Scan Rust content for over-engineering heuristics.
/// Returns Some([`SIMPLICITY_ADVISORY`]) if signals found, None otherwise.
pub(crate) fn advisory(file_path: &str, content: &str) -> Option<String> {
    if !std::path::Path::new(file_path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
    {
        return None;
    }
    let signals = detect_signals(content);
    if signals.is_empty() {
        return None;
    }
    let mut msg = String::from("[SIMPLICITY_ADVISORY]\n");
    for signal in &signals {
        use std::fmt::Write as _;
        writeln!(msg, "  {signal}").ok();
    }
    msg.push_str("action: Minimum code that solves the problem. Nothing speculative.\n");
    Some(msg)
}

fn detect_signals(content: &str) -> Vec<&'static str> {
    let mut signals: Vec<&'static str> = Vec::new();
    // Builder pattern for likely small structs (≤3 setter methods heuristic)
    if content.contains("fn build(") && content.contains("Builder") {
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "sum of two match counts, bounded by content size"
        )]
        let setter_count =
            content.matches("pub fn set_").count() + content.matches("pub fn with_").count();
        if setter_count <= 3 {
            signals.push("Builder pattern for small struct — use plain constructor instead");
        }
    }
    // Feature flags in app context
    if content.contains("#[cfg(feature =") {
        signals.push("#[cfg(feature)] detected — feature flags add speculative complexity");
    }
    // Trait with single impl in same file
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "sum of two match counts, bounded by content size"
    )]
    let trait_count = content.matches("\ntrait ").count() + content.matches("\npub trait ").count();
    let impl_count = content.matches("\nimpl ").count();
    if trait_count >= 1 && impl_count == 1 {
        signals.push("Trait with single impl — use concrete type until second impl exists");
    }
    // Phantom type parameters (speculative generics)
    if content.contains("PhantomData") {
        signals.push("PhantomData — verify this generic variance is actually needed");
    }
    signals
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_none_for_non_rust_file() {
        assert!(advisory("Component.tsx", "trait Foo {}").is_none());
    }

    #[test]
    fn should_return_none_for_clean_rust_code() {
        let clean = "pub struct Config { pub host: String, pub port: u16 }\n";
        assert!(advisory("src/config.rs", clean).is_none());
    }

    #[test]
    fn should_detect_small_builder_pattern() {
        let code = "struct FooBuilder {}\nimpl FooBuilder {\n  pub fn with_x(self) -> Self { self }\n  fn build(self) -> Foo { Foo {} }\n}\n";
        let result = advisory("src/builder.rs", code);
        assert!(result.is_some());
    }

    #[test]
    fn should_not_flag_large_builder() {
        let code = "struct FooBuilder {}\nimpl FooBuilder {\n  pub fn with_a(self) -> Self { self }\n  pub fn with_b(self) -> Self { self }\n  pub fn with_c(self) -> Self { self }\n  pub fn with_d(self) -> Self { self }\n  fn build(self) -> Foo { Foo {} }\n}\n";
        assert!(advisory("src/builder.rs", code).is_none());
    }

    #[test]
    fn should_detect_cfg_feature() {
        let code = "#[cfg(feature = \"experimental\")]\npub fn foo() {}\n";
        assert!(advisory("src/lib.rs", code).is_some());
    }

    #[test]
    fn should_detect_phantom_data() {
        let code = "use std::marker::PhantomData;\nstruct Typed<T> { _t: PhantomData<T> }\n";
        let result = advisory("src/typed.rs", code);
        assert!(result.is_some());
        let s = result.unwrap_or_default();
        assert!(s.contains("[SIMPLICITY_ADVISORY]"));
    }

    #[test]
    fn should_include_action_line_in_output() {
        let code = "use std::marker::PhantomData;\nstruct Typed<T> { _t: PhantomData<T> }\n";
        let result = advisory("src/typed.rs", code);
        let s = result.unwrap_or_default();
        assert!(s.contains("action:"));
    }
}

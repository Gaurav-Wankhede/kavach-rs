//! Dioxus 0.7.7 Production Gate — Atomic UI + Anti-Pattern Detection
//!
//! SOURCES (verified 2026-05):
//! - <https://dioxuslabs.com/learn/0.7/guides/tips/antipatterns>/
//! - <https://dioxuslabs.com/learn/0.7/essentials/advanced/lifecycle>/ (`use_drop`)
//! - <https://docs.rs/dioxus/latest/dioxus/prelude/struct.Signal.html>
//! - <https://github.com/DioxusLabs/dioxus/issues/3526> (mem leak: rapid re-renders)
//! - <https://github.com/DioxusLabs/dioxus/issues/3421> (event handler leak)
//! - <https://atomicdesign.bradfrost.com/chapter-2>/

use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DioxusSeverity {
    P0Block,
    P1Advisory,
    P2Warning,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct DioxusViolation {
    pub severity: DioxusSeverity,
    pub pattern: &'static str,
    pub fix: &'static str,
    pub line: usize,
}

static PATTERN_0: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?m)^\s*(?:count|state|signal|value)\s*\+=").ok());
static PATTERN_1: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"key:\s*(?:i\b|idx\b|index\b|\{i\}|\{idx\}|\{index\})").ok());
static PATTERN_2: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"(?:if|match)[^{]*\{[^}]*use_(?:signal|state|memo|effect|context|resource)").ok()
});
static PATTERN_3: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?:static|const)\s+\w+\s*:\s*GlobalSignal").ok());
static PATTERN_4: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"impl\s+PartialEq\s+for\s+\w+Props[^}]*\{\s*fn\s+eq[^}]*true\s*\}").ok()
});
static PATTERN_5: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"use_signal\(\|\|\s*(?:vec!\[\]|Vec::new\(\)|HashMap::new\(\)|String::new\(\))")
        .ok()
});
static PATTERN_6: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"web_sys::(?:document|window)\(\)").ok());
static PATTERN_7: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?:std::thread::sleep|futures::executor::block_on)").ok());
static PATTERN_8: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(
        r"(?:bg|text|border)-(?:red|blue|green|yellow|purple|pink|gray|indigo|amber)-\d{2,3}",
    )
    .ok()
});
static PATTERN_9: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"->\s*Element\s*\{[^}]*(?:html!|view!|dom!)\(").ok());
static PATTERN_10: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"class:\s*"[^"]*\bbg-white\b[^"]*""#).ok());

fn get_pattern(index: usize) -> Option<&'static Regex> {
    match index {
        1 => PATTERN_1.as_ref(),
        2 => PATTERN_2.as_ref(),
        3 => PATTERN_3.as_ref(),
        4 => PATTERN_4.as_ref(),
        5 => PATTERN_5.as_ref(),
        6 => PATTERN_6.as_ref(),
        7 => PATTERN_7.as_ref(),
        8 => PATTERN_8.as_ref(),
        9 => PATTERN_9.as_ref(),
        10 => PATTERN_10.as_ref(),
        _ => PATTERN_0.as_ref(),
    }
}

/// Detect Dioxus 0.7.7 anti-patterns and atomic UI violations
#[must_use]
pub fn detect(file_path: &str, content: &str) -> Vec<DioxusViolation> {
    if !is_dioxus_file(file_path, content) {
        return vec![];
    }
    if crate::file_types::is_test_file(file_path) {
        return vec![];
    }

    let is_lib = file_path.ends_with("lib.rs");
    let is_atom = file_path.contains("/atoms/") || file_path.ends_with("atoms.rs");
    let is_molecule = file_path.contains("/molecules/") || file_path.ends_with("molecules.rs");
    let is_organism = file_path.contains("/organisms/") || file_path.ends_with("organisms.rs");

    let mut violations = detect_render_mutations(content);
    violations.extend(detect_hook_violations(content, is_lib));
    violations.extend(detect_dom_violations(content));
    violations.extend(detect_hierarchy_violations(content, is_atom, is_molecule));
    violations.extend(detect_style_violations(content));
    violations.extend(detect_classification_violations(
        content,
        file_path,
        is_atom,
        is_molecule,
        is_organism,
    ));

    violations
}

fn detect_render_mutations(content: &str) -> Vec<DioxusViolation> {
    let mut violations = Vec::new();
    let pattern_0 = get_pattern(0);
    for (i, line) in content.lines().enumerate() {
        if pattern_0.is_some_and(|re| re.is_match(line))
            && !line.contains("onclick:")
            && !line.contains("oninput:")
            && !line.contains("onchange:")
            && !line.contains("use_effect")
            && !line.contains("spawn")
            && !line.trim_start().starts_with("//")
        {
            violations.push(DioxusViolation {
                severity: DioxusSeverity::P0Block,
                pattern: "state update during render",
                fix: "Move to event handler or use_effect. Updates during render cause infinite loops.",
                line: i.saturating_add(1),
            });
        }
    }
    violations
}

fn detect_hook_violations(content: &str, is_lib: bool) -> Vec<DioxusViolation> {
    let mut violations = Vec::new();
    if get_pattern(2).is_some_and(|re| re.is_match(content)) {
        violations.push(DioxusViolation {
            severity: DioxusSeverity::P0Block,
            pattern: "conditional hook",
            fix: "Hooks must run unconditionally at component top. Move use_signal/use_effect outside if/match.",
            line: 0,
        });
    }
    if is_lib && get_pattern(3).is_some_and(|re| re.is_match(content)) {
        violations.push(DioxusViolation {
            severity: DioxusSeverity::P0Block,
            pattern: "GlobalSignal in library",
            fix: "Use use_context_provider in libraries. GlobalSignal prevents component reuse across instances.",
            line: 0,
        });
    }
    violations
}

fn detect_dom_violations(content: &str) -> Vec<DioxusViolation> {
    let mut violations = Vec::new();
    if get_pattern(7).is_some_and(|re| re.is_match(content)) {
        violations.push(DioxusViolation {
            severity: DioxusSeverity::P0Block,
            pattern: "blocking in component",
            fix: "Use tokio::time::sleep + spawn(async move). Blocking freezes the UI thread.",
            line: 0,
        });
    }
    if get_pattern(9).is_some_and(|re| re.is_match(content)) {
        violations.push(DioxusViolation {
            severity: DioxusSeverity::P0Block,
            pattern: "wrong UI macro",
            fix: "Dioxus 0.7 uses rsx! macro, not html!/view!/dom!.",
            line: 0,
        });
    }
    if get_pattern(1).is_some_and(|re| re.is_match(content)) {
        violations.push(DioxusViolation {
            severity: DioxusSeverity::P1Advisory,
            pattern: "index as list key",
            fix: "Use stable id: key: \"{item.id}\". Index keys break diffing on reorder.",
            line: 0,
        });
    }
    if get_pattern(4).is_some_and(|re| re.is_match(content)) {
        violations.push(DioxusViolation {
            severity: DioxusSeverity::P1Advisory,
            pattern: "PartialEq always true",
            fix: "Props PartialEq must return false when UI would change, else child stays stale.",
            line: 0,
        });
    }
    if get_pattern(5).is_some_and(|re| re.is_match(content)) {
        violations.push(DioxusViolation {
            severity: DioxusSeverity::P1Advisory,
            pattern: "use_signal without type",
            fix: "Add explicit type: use_signal::<Vec<T>>(Vec::new). Empty collections fail inference.",
            line: 0,
        });
    }
    if get_pattern(6).is_some_and(|re| re.is_match(content)) {
        violations.push(DioxusViolation {
            severity: DioxusSeverity::P1Advisory,
            pattern: "direct DOM manipulation",
            fix: "Use Dioxus signals + rsx!. web_sys breaks reactivity and cross-platform support.",
            line: 0,
        });
    }
    violations
}

fn detect_hierarchy_violations(
    content: &str,
    is_atom: bool,
    is_molecule: bool,
) -> Vec<DioxusViolation> {
    let mut violations = Vec::new();
    if is_atom && (content.contains("molecules::") || content.contains("organisms::")) {
        violations.push(DioxusViolation {
            severity: DioxusSeverity::P1Advisory,
            pattern: "atom importing molecule/organism",
            fix: "Atoms are leaf components. Cannot depend on molecules or organisms.",
            line: 0,
        });
    }
    if is_molecule && content.contains("organisms::") {
        violations.push(DioxusViolation {
            severity: DioxusSeverity::P1Advisory,
            pattern: "molecule importing organism",
            fix: "Molecules compose atoms only. Move organism dependency to page level.",
            line: 0,
        });
    }
    violations
}

fn detect_style_violations(content: &str) -> Vec<DioxusViolation> {
    let mut violations = Vec::new();
    if get_pattern(8).is_some_and(|re| re.is_match(content)) {
        violations.push(DioxusViolation {
            severity: DioxusSeverity::P2Warning,
            pattern: "hardcoded color",
            fix: "Use theme tokens (theme.primary, ring-1 ring-white/10). Hardcoded colors break theming.",
            line: 0,
        });
    }
    if get_pattern(10).is_some_and(|re| re.is_match(content)) && !content.contains("dark:") {
        violations.push(DioxusViolation {
            severity: DioxusSeverity::P2Warning,
            pattern: "missing dark mode",
            fix: "Pair every light class with dark: variant. bg-white needs dark:bg-gray-900.",
            line: 0,
        });
    }
    violations
}

fn detect_classification_violations(
    content: &str,
    file_path: &str,
    is_atom: bool,
    is_molecule: bool,
    is_organism: bool,
) -> Vec<DioxusViolation> {
    let mut violations = Vec::new();
    let is_component = content.contains("#[component]") || content.contains("-> Element");
    let is_categorized = is_atom
        || is_molecule
        || is_organism
        || file_path.contains("pages/")
        || file_path.contains("templates/");
    if is_component && !is_categorized {
        let has_level = content.contains("// ATOM:")
            || content.contains("// MOLECULE:")
            || content.contains("// ORGANISM:")
            || content.contains("// PAGE:")
            || content.contains("// TEMPLATE:");
        if !has_level {
            violations.push(DioxusViolation {
                severity: DioxusSeverity::P2Warning,
                pattern: "missing atomic level comment",
                fix: "Add // ATOM:, // MOLECULE:, or // ORGANISM: to classify component level.",
                line: 0,
            });
        }
    }
    violations
}

fn is_dioxus_file(path: &str, content: &str) -> bool {
    if !std::path::Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
    {
        return false;
    }
    content.contains("dioxus")
        || content.contains("rsx!")
        || content.contains("-> Element")
        || content.contains("#[component]")
        || content.contains("use_signal")
        || content.contains("GlobalSignal")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_state_update_during_render() {
        let code = "fn Counter() -> Element {\n    let mut count = use_signal(|| 0);\n    count += 1;\n    rsx! { \"{count}\" }\n}";
        let v = detect("src/components/counter.rs", code);
        assert!(v.iter().any(|x| x.pattern == "state update during render"));
    }

    #[test]
    fn allows_state_update_in_handler() {
        let code = "fn Counter() -> Element {\n    let mut count = use_signal(|| 0);\n    rsx! { button { onclick: move |_| count += 1, \"Click\" } }\n}";
        let v = detect("src/components/counter.rs", code);
        assert!(!v.iter().any(|x| x.pattern == "state update during render"));
    }

    #[test]
    fn detects_conditional_hook() {
        let code = "fn Component(show: bool) -> Element {\n    if show { let signal = use_signal(|| 0); }\n    rsx! { \"hello\" }\n}";
        let v = detect("src/components/cond.rs", code);
        assert!(v.iter().any(|x| x.pattern == "conditional hook"));
    }

    #[test]
    fn detects_index_as_key() {
        let code =
            "rsx! { for (i, item) in items.iter().enumerate() { li { key: i, \"{item}\" } } }";
        let v = detect("src/components/list.rs", code);
        assert!(v.iter().any(|x| x.pattern == "index as list key"));
    }

    #[test]
    fn allows_id_as_key() {
        let code =
            "rsx! { for item in items.iter() { li { key: \"{item.id}\", \"{item.name}\" } } }";
        let v = detect("src/components/list.rs", code);
        assert!(!v.iter().any(|x| x.pattern == "index as list key"));
    }

    #[test]
    fn detects_global_signal_in_lib() {
        let code = "static THEME: GlobalSignal<Theme> = Signal::global(|| Theme::Light);";
        let v = detect("src/lib.rs", code);
        assert!(v.iter().any(|x| x.pattern == "GlobalSignal in library"));
    }

    #[test]
    fn allows_global_signal_in_app() {
        let code = "static THEME: GlobalSignal<Theme> = Signal::global(|| Theme::Light);\nfn App() -> Element { rsx! {} }";
        let v = detect("src/main.rs", code);
        assert!(!v.iter().any(|x| x.pattern == "GlobalSignal in library"));
    }

    #[test]
    fn detects_blocking_call() {
        let code = "fn Slow() -> Element {\n    std::thread::sleep(Duration::from_secs(1));\n    rsx! { \"done\" }\n}";
        let v = detect("src/components/slow.rs", code);
        assert!(v.iter().any(|x| x.pattern == "blocking in component"));
    }

    #[test]
    fn detects_atom_importing_organism() {
        let code =
            "use crate::organisms::Header;\npub fn Button() -> Element { rsx! { Header {} } }";
        let v = detect("src/ui/atoms/button.rs", code);
        assert!(
            v.iter()
                .any(|x| x.pattern == "atom importing molecule/organism")
        );
    }

    #[test]
    fn detects_molecule_importing_organism() {
        let code = "use crate::organisms::DataTable;\npub fn SearchBar() -> Element { rsx! { DataTable {} } }";
        let v = detect("src/ui/molecules/search_bar.rs", code);
        assert!(v.iter().any(|x| x.pattern == "molecule importing organism"));
    }

    #[test]
    fn allows_molecule_importing_atom() {
        let code = "use crate::atoms::{Button, Input};\npub fn SearchBar() -> Element { rsx! { Input {} Button { \"Search\" } } }";
        let v = detect("src/ui/molecules/search_bar.rs", code);
        assert!(!v.iter().any(|x| x.pattern.contains("importing")));
    }

    #[test]
    fn detects_hardcoded_color() {
        let code = "fn Card() -> Element { rsx! { div { class: \"bg-blue-500 text-white\" } } }";
        let v = detect("src/components/card.rs", code);
        assert!(v.iter().any(|x| x.pattern == "hardcoded color"));
    }

    #[test]
    fn detects_wrong_macro() {
        let code = "fn Bad() -> Element { html!(div { \"hi\" }) }";
        let v = detect("src/components/bad.rs", code);
        assert!(v.iter().any(|x| x.pattern == "wrong UI macro"));
    }

    #[test]
    fn test_file_skipped() {
        let code = "fn test() -> Element {\n    count += 1;\n    rsx! { \"\" }\n}";
        let v = detect("/project/tests/ui_test.rs", code);
        assert!(v.is_empty());
    }

    #[test]
    fn non_dioxus_file_skipped() {
        let code = "fn main() { println!(\"Hello\"); }";
        let v = detect("src/cli.rs", code);
        assert!(v.is_empty());
    }
}

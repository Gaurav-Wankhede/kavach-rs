// split: Single-module gate file for framework-agnostic atomic UI enforcement.
//! Atomic UI Production Gate — Framework-Agnostic
//!
//! Enforces Brad Frost's Atomic Design (5 chapters) across React, Vue, Svelte,
//! Solid, Astro, Dioxus, Yew, Leptos. Aligned with 2026 Contract-Driven Design
//! evolution — atoms/molecules/organisms structure becomes an enforceable contract.
//!
//! HIERARCHY: Pages → Templates → Organisms → Molecules → Atoms → (Tokens)
//!
//! IMPORT CONTRACT:
//!   Atoms     ← tokens, std primitives only
//!   Molecules ← atoms, tokens
//!   Organisms ← molecules, atoms, tokens
//!   Templates ← organisms, molecules, atoms, tokens
//!   Pages     ← anything
//!
//! SOURCES (verified 2026-05):
//! - <https://atomicdesign.bradfrost.com/table-of-contents>/
//! - <https://atomicdesign.bradfrost.com/chapter-1>/
//! - <https://atomicdesign.bradfrost.com/chapter-2>/
//! - <https://atomicdesign.bradfrost.com/chapter-3>/
//! - <https://atomicdesign.bradfrost.com/chapter-4>/
//! - <https://atomicdesign.bradfrost.com/chapter-5>/
//! - <https://designtokenscourse.com>/
//! - <https://aianddesign.systems/#content>
//! - <https://atomicdesigncourse.com>/
//! - <https://medium.com/@iz.iuqo/atomic-design-reached-its-peak-contract-driven-design-is-what-comes-next-9174a9a89aea>

use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AtomicSeverity {
    P0Block,
    P1Advisory,
    P2Warning,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct AtomicViolation {
    pub severity: AtomicSeverity,
    pub pattern: &'static str,
    pub fix: &'static str,
    pub line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Level {
    Atom,
    Molecule,
    Organism,
    Template,
    Page,
    Unknown,
}

fn regex_matches(opt_re: Option<&Option<Regex>>, text: &str) -> bool {
    opt_re
        .and_then(|o| o.as_ref())
        .is_some_and(|re| re.is_match(text))
}

fn regex_find_any(
    opt_re: Option<&Option<Regex>>,
    text: &str,
    predicate: impl Fn(&str) -> bool,
) -> bool {
    opt_re
        .and_then(|o| o.as_ref())
        .is_some_and(|re| re.find_iter(text).any(|m| predicate(m.as_str())))
}

fn classify_path(path: &str) -> Level {
    let p = path.to_lowercase();
    if p.contains("/atoms/")
        || p.contains("\\atoms\\")
        || p.ends_with("/atoms.rs")
        || p.contains("/atom/")
    {
        return Level::Atom;
    }
    if p.contains("/molecules/")
        || p.contains("\\molecules\\")
        || p.ends_with("/molecules.rs")
        || p.contains("/molecule/")
    {
        return Level::Molecule;
    }
    if p.contains("/organisms/")
        || p.contains("\\organisms\\")
        || p.ends_with("/organisms.rs")
        || p.contains("/organism/")
    {
        return Level::Organism;
    }
    if p.contains("/templates/") || p.contains("\\templates\\") {
        return Level::Template;
    }
    if p.contains("/pages/") || p.contains("\\pages\\") || p.contains("/routes/") {
        return Level::Page;
    }
    Level::Unknown
}

static PATTERNS: LazyLock<Vec<Option<Regex>>> = LazyLock::new(|| {
    vec![
        Regex::new(r#"(?m)(?:import|from|use)\s+[^;]*['"]?\.{0,2}/?(?:[\w-]+/)*molecules?(?:/|['"]|::)"#).ok(),
        Regex::new(r#"(?m)(?:import|from|use)\s+[^;]*['"]?\.{0,2}/?(?:[\w-]+/)*organisms?(?:/|['"]|::)"#).ok(),
        Regex::new(r#"(?m)(?:import|from|use)\s+[^;]*['"]?\.{0,2}/?(?:[\w-]+/)*templates?(?:/|['"]|::)"#).ok(),
        Regex::new(r#"(?m)(?:import|from|use)\s+[^;]*['"]?\.{0,2}/?(?:[\w-]+/)*pages?(?:/|['"]|::)"#).ok(),
        Regex::new(r"(?:useStore|useSelector|useDispatch|useAtom|useRecoilState|useRecoilValue|defineStore|createStore|GlobalSignal|use_global)").ok(),
        Regex::new(r"(?:fetch\s*\(|axios\.(?:get|post|put|patch|delete)|ky\.(?:get|post|put|delete)|\$fetch\s*\(|reqwest::|surf::|reqwasm::)").ok(),
        Regex::new(r"style\s*=\s*\{?\{[^}]*(?:#[0-9a-fA-F]{3,8}|rgb\s*\(|rgba\s*\(|hsl\s*\()").ok(),
        Regex::new(r#"(?:className|class|class:list)\s*=\s*[`"'][^`"']*\[\d+px\]"#).ok(),
        Regex::new(r"<img\b[^>]*>").ok(),
        Regex::new(r"<button\b[^>]*>\s*(?:<svg|<i\s+class|<Icon\b)").ok(),
        Regex::new(r"(?s)(?:\.map\s*\(|v-for|#each|\{#each)[^<]*<[a-zA-Z]\w*[^>]*>").ok(),
        Regex::new(r#"(?:className|class|color|bg|background)\s*[:=]\s*[`"']?[^`"']*#[0-9a-fA-F]{3,8}\b"#).ok(),
        Regex::new(r#"(?:className|class)\s*=\s*[`"'][^`"']*\bbg-white\b[^`"']*[`"']"#).ok(),
        Regex::new(r"(?:localStorage|sessionStorage)\s*\.\s*(?:setItem|getItem|removeItem)").ok(),
        Regex::new(r"(?:console\.(?:log|debug|trace)|tracing::debug!|log::debug!)").ok(),
    ]
});

fn detect_atom_violations(content: &str) -> Vec<AtomicViolation> {
    let mut v = Vec::new();
    if regex_matches(PATTERNS.first(), content) {
        v.push(AtomicViolation {
            severity: AtomicSeverity::P0Block,
            pattern: "atom imports molecule",
            fix: "Atoms are leaf primitives. Cannot depend on molecules.",
            line: 0,
        });
    }
    if regex_matches(PATTERNS.get(1), content) {
        v.push(AtomicViolation {
            severity: AtomicSeverity::P0Block,
            pattern: "atom imports organism",
            fix: "Atoms cannot depend on organisms. Atoms compose atoms only.",
            line: 0,
        });
    }
    if regex_matches(PATTERNS.get(2), content) || regex_matches(PATTERNS.get(3), content) {
        v.push(AtomicViolation {
            severity: AtomicSeverity::P0Block,
            pattern: "atom imports template/page",
            fix: "Atoms must not import templates or pages.",
            line: 0,
        });
    }
    if regex_matches(PATTERNS.get(4), content) {
        v.push(AtomicViolation {
            severity: AtomicSeverity::P0Block,
            pattern: "atom uses state store",
            fix: "Atoms must be stateless and prop-driven.",
            line: 0,
        });
    }
    if regex_matches(PATTERNS.get(5), content) {
        v.push(AtomicViolation {
            severity: AtomicSeverity::P0Block,
            pattern: "atom calls API",
            fix: "Atoms must not fetch data. Pass via props.",
            line: 0,
        });
    }
    if regex_matches(PATTERNS.get(13), content) {
        v.push(AtomicViolation {
            severity: AtomicSeverity::P1Advisory,
            pattern: "atom uses storage",
            fix: "Atoms must be pure. localStorage belongs in organisms or hooks.",
            line: 0,
        });
    }
    v
}

fn detect_molecule_violations(content: &str) -> Vec<AtomicViolation> {
    let mut v = Vec::new();
    if regex_matches(PATTERNS.get(1), content) {
        v.push(AtomicViolation {
            severity: AtomicSeverity::P0Block,
            pattern: "molecule imports organism",
            fix: "Molecules compose atoms only.",
            line: 0,
        });
    }
    if regex_matches(PATTERNS.get(2), content) || regex_matches(PATTERNS.get(3), content) {
        v.push(AtomicViolation {
            severity: AtomicSeverity::P0Block,
            pattern: "molecule imports template/page",
            fix: "Molecules cannot depend on templates/pages.",
            line: 0,
        });
    }
    if regex_matches(PATTERNS.get(5), content) {
        v.push(AtomicViolation {
            severity: AtomicSeverity::P1Advisory,
            pattern: "molecule calls API",
            fix: "API calls belong at organism/page boundary.",
            line: 0,
        });
    }
    v
}

fn detect_organism_violations(content: &str) -> Vec<AtomicViolation> {
    let mut v = Vec::new();
    if regex_matches(PATTERNS.get(2), content) || regex_matches(PATTERNS.get(3), content) {
        v.push(AtomicViolation {
            severity: AtomicSeverity::P0Block,
            pattern: "organism imports template/page",
            fix: "Organisms cannot depend on templates/pages.",
            line: 0,
        });
    }
    v
}

fn detect_accessibility_violations(content: &str) -> Vec<AtomicViolation> {
    let mut v = Vec::new();
    if regex_find_any(PATTERNS.get(8), content, |m| {
        !m.contains("alt=") && !m.contains("alt =")
    }) {
        v.push(AtomicViolation {
            severity: AtomicSeverity::P0Block,
            pattern: "img without alt",
            fix: "Every <img> must have alt=\"...\". WCAG 1.1.1.",
            line: 0,
        });
    }
    if regex_find_any(PATTERNS.get(9), content, |m| !m.contains("aria-label")) {
        v.push(AtomicViolation {
            severity: AtomicSeverity::P0Block,
            pattern: "icon button without aria-label",
            fix: "Icon-only buttons need aria-label. WCAG 4.1.2.",
            line: 0,
        });
    }
    if regex_find_any(PATTERNS.get(10), content, |m| {
        !m.contains("key=") && !m.contains("key ")
    }) {
        v.push(AtomicViolation {
            severity: AtomicSeverity::P0Block,
            pattern: "list without key",
            fix: "List rendering must include key={item.id}.",
            line: 0,
        });
    }
    v
}

fn detect_design_token_violations(content: &str) -> Vec<AtomicViolation> {
    let mut v = Vec::new();
    if regex_matches(PATTERNS.get(6), content) {
        v.push(AtomicViolation {
            severity: AtomicSeverity::P1Advisory,
            pattern: "inline style with hardcoded color",
            fix: "Use design tokens via Tailwind or CSS variables.",
            line: 0,
        });
    }
    if regex_matches(PATTERNS.get(7), content) {
        v.push(AtomicViolation {
            severity: AtomicSeverity::P1Advisory,
            pattern: "arbitrary px value",
            fix: "Use design-system spacing scale.",
            line: 0,
        });
    }
    if regex_matches(PATTERNS.get(11), content) {
        v.push(AtomicViolation {
            severity: AtomicSeverity::P1Advisory,
            pattern: "hardcoded hex in className",
            fix: "Use theme tokens. Hardcoded hex breaks dark mode.",
            line: 0,
        });
    }
    if regex_matches(PATTERNS.get(12), content) && !content.contains("dark:") {
        v.push(AtomicViolation {
            severity: AtomicSeverity::P2Warning,
            pattern: "missing dark mode pairing",
            fix: "bg-white needs dark:bg-gray-900.",
            line: 0,
        });
    }
    v
}

fn detect_debug_violations(content: &str) -> Vec<AtomicViolation> {
    let mut v = Vec::new();
    if regex_matches(PATTERNS.get(14), content) {
        v.push(AtomicViolation {
            severity: AtomicSeverity::P2Warning,
            pattern: "debug logging in component",
            fix: "Remove console.log/tracing::debug! before commit.",
            line: 0,
        });
    }
    v
}

#[must_use]
pub fn detect(file_path: &str, content: &str) -> Vec<AtomicViolation> {
    if !is_ui_file(file_path, content) {
        return vec![];
    }
    if crate::file_types::is_test_file(file_path) {
        return vec![];
    }

    let mut v = Vec::new();
    let level = classify_path(file_path);

    match level {
        Level::Atom => v.extend(detect_atom_violations(content)),
        Level::Molecule => v.extend(detect_molecule_violations(content)),
        Level::Organism => v.extend(detect_organism_violations(content)),
        _ => {}
    }

    v.extend(detect_accessibility_violations(content));
    v.extend(detect_design_token_violations(content));
    v.extend(detect_debug_violations(content));

    v
}

fn is_ui_file(path: &str, content: &str) -> bool {
    let p = path.to_lowercase();
    let ext = Path::new(&p).extension().and_then(|e| e.to_str());
    if matches!(ext, Some("tsx" | "jsx" | "vue" | "svelte" | "astro"))
        || (ext == Some("ts") && (p.contains("/components/") || p.contains("/ui/")))
        || (ext == Some("js") && (p.contains("/components/") || p.contains("/ui/")))
        || (ext == Some("rs")
            && (p.contains("/components/")
                || p.contains("/ui/")
                || p.contains("/atoms/")
                || p.contains("/molecules/")
                || p.contains("/organisms/")))
    {
        return true;
    }
    // Content-based Dioxus detection: rsx! macro invocation or #[component] attribute.
    // Strips line comments to avoid false positives in `// rsx! note` or doc strings.
    if ext != Some("rs") {
        return false;
    }
    let stripped: String = content
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    DIOXUS_MARKER
        .as_ref()
        .is_some_and(|re| re.is_match(&stripped))
        || stripped.contains("dioxus::prelude")
}

static DIOXUS_MARKER: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"\brsx!\s*[{(\[]|#\[component\]").ok());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_atom_importing_molecule_react() {
        let v = detect(
            "src/ui/atoms/Button.tsx",
            "import { SearchBar } from '../molecules/SearchBar';",
        );
        assert!(v.iter().any(|x| x.pattern == "atom imports molecule"));
    }
    #[test]
    fn detects_atom_importing_organism_vue() {
        let v = detect(
            "src/atoms/Logo.vue",
            "<script>import H from '@/organisms/Header.vue';</script>",
        );
        assert!(v.iter().any(|x| x.pattern == "atom imports organism"));
    }
    #[test]
    fn detects_atom_importing_organism_svelte() {
        let v = detect(
            "src/atoms/Cell.svelte",
            "<script>import D from '../organisms/DataTable.svelte';</script>",
        );
        assert!(v.iter().any(|x| x.pattern == "atom imports organism"));
    }
    #[test]
    fn detects_atom_importing_organism_dioxus() {
        let v = detect(
            "src/ui/atoms/button.rs",
            "use crate::organisms::Header; pub fn Button() -> Element { rsx! {} }",
        );
        assert!(v.iter().any(|x| x.pattern == "atom imports organism"));
    }
    #[test]
    fn detects_atom_using_state_store() {
        let v = detect(
            "src/atoms/Button.tsx",
            "import { useStore } from '@/store'; export const Button = () => { useStore(); return <button />; };",
        );
        assert!(v.iter().any(|x| x.pattern == "atom uses state store"));
    }
    #[test]
    fn detects_atom_calling_api() {
        let v = detect(
            "src/atoms/Avatar.tsx",
            "export const Avatar = async () => { await fetch('/api/user'); return <img alt='' />; };",
        );
        assert!(v.iter().any(|x| x.pattern == "atom calls API"));
    }
    #[test]
    fn detects_molecule_importing_organism() {
        let v = detect(
            "src/molecules/SearchBar.tsx",
            "import H from '../organisms/Header'; export const S = () => <H />;",
        );
        assert!(v.iter().any(|x| x.pattern == "molecule imports organism"));
    }
    #[test]
    fn allows_molecule_importing_atom() {
        let v = detect(
            "src/molecules/Form.tsx",
            "import { Button } from '../atoms/Button'; export const F = () => <Button alt='' />;",
        );
        assert!(!v.iter().any(|x| x.pattern.contains("imports")));
    }
    #[test]
    fn detects_organism_importing_template() {
        let v = detect(
            "src/organisms/Header.tsx",
            "import { M } from '../templates/MainTemplate'; export const H = () => <M />;",
        );
        assert!(
            v.iter()
                .any(|x| x.pattern == "organism imports template/page")
        );
    }
    #[test]
    fn detects_img_without_alt() {
        let v = detect(
            "src/components/Avatar.tsx",
            "export const A = () => <img src='/a.png' />;",
        );
        assert!(v.iter().any(|x| x.pattern == "img without alt"));
    }
    #[test]
    fn allows_img_with_alt() {
        let v = detect(
            "src/components/Avatar.tsx",
            "export const A = () => <img src='/a.png' alt='User' />;",
        );
        assert!(!v.iter().any(|x| x.pattern == "img without alt"));
    }
    #[test]
    fn detects_icon_button_without_aria_label() {
        let v = detect(
            "src/components/Close.tsx",
            "export const C = () => <button onClick={x}><svg /></button>;",
        );
        assert!(
            v.iter()
                .any(|x| x.pattern == "icon button without aria-label")
        );
    }
    #[test]
    fn allows_icon_button_with_aria_label() {
        let v = detect(
            "src/components/Close.tsx",
            "export const C = () => <button aria-label='Close' onClick={x}><svg /></button>;",
        );
        assert!(
            !v.iter()
                .any(|x| x.pattern == "icon button without aria-label")
        );
    }
    #[test]
    fn detects_list_without_key() {
        let v = detect(
            "src/components/List.tsx",
            "export const L = ({items}) => items.map(x => <li>{x}</li>);",
        );
        assert!(v.iter().any(|x| x.pattern == "list without key"));
    }
    #[test]
    fn allows_list_with_key() {
        let v = detect(
            "src/components/List.tsx",
            "export const L = ({items}) => items.map(x => <li key={x.id}>{x.name}</li>);",
        );
        assert!(!v.iter().any(|x| x.pattern == "list without key"));
    }
    #[test]
    fn detects_inline_style_with_hex() {
        let v = detect(
            "src/components/Card.tsx",
            "export const C = () => <div alt='' style={{color:'#ff0000'}}>x</div>;",
        );
        assert!(
            v.iter()
                .any(|x| x.pattern == "inline style with hardcoded color")
        );
    }
    #[test]
    fn detects_arbitrary_px() {
        let v = detect(
            "src/components/Box.tsx",
            "export const B = () => <div alt='' className='p-[13px]'>x</div>;",
        );
        assert!(v.iter().any(|x| x.pattern == "arbitrary px value"));
    }
    #[test]
    fn detects_missing_dark_mode() {
        let v = detect(
            "src/components/Card.tsx",
            "export const C = () => <div alt='' className='bg-white text-black'>x</div>;",
        );
        assert!(v.iter().any(|x| x.pattern == "missing dark mode pairing"));
    }
    #[test]
    fn allows_paired_dark_mode() {
        let v = detect(
            "src/components/Card.tsx",
            "export const C = () => <div alt='' className='bg-white dark:bg-gray-900'>x</div>;",
        );
        assert!(!v.iter().any(|x| x.pattern == "missing dark mode pairing"));
    }
    #[test]
    fn detects_atom_using_localstorage() {
        let v = detect(
            "src/atoms/Theme.tsx",
            "export const T = () => { const t = localStorage.getItem('theme'); return <span>{t}</span>; };",
        );
        assert!(v.iter().any(|x| x.pattern == "atom uses storage"));
    }
    #[test]
    fn detects_console_log() {
        let v = detect(
            "src/components/Card.tsx",
            "export const C = () => { console.log('x'); return <div alt='' />; };",
        );
        assert!(v.iter().any(|x| x.pattern == "debug logging in component"));
    }
    #[test]
    fn non_ui_file_skipped() {
        let v = detect(
            "src/utils/math.ts",
            "export const add = (a:number,b:number) => a+b;",
        );
        assert!(v.is_empty());
    }
    #[test]
    fn test_file_skipped() {
        let v = detect(
            "/project/tests/Button.test.tsx",
            "import x from '../organisms/y';",
        );
        assert!(v.is_empty());
    }
}

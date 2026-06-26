use super::types::{AtomicSeverity, AtomicViolation, Level};
use super::util::{PATTERNS, regex_find_any, regex_matches};

pub(super) fn detect_atom_violations(content: &str) -> Vec<AtomicViolation> {
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

pub(super) fn detect_molecule_violations(content: &str) -> Vec<AtomicViolation> {
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

pub(super) fn detect_organism_violations(content: &str) -> Vec<AtomicViolation> {
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

pub(super) fn detect_accessibility_violations(content: &str) -> Vec<AtomicViolation> {
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

pub(super) fn detect_design_token_violations(content: &str) -> Vec<AtomicViolation> {
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

pub(super) fn detect_debug_violations(content: &str) -> Vec<AtomicViolation> {
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

pub(super) fn dispatch(_file_path: &str, content: &str, level: Level) -> Vec<AtomicViolation> {
    let mut v = Vec::new();
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

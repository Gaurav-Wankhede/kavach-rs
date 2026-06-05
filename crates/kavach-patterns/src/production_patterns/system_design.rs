//! System design anti-patterns.

use super::types::{Severity, mk};

pub(super) fn build() -> Vec<(Option<regex::Regex>, &'static str, &'static str, Severity)> {
    vec![
        (
            mk(r"(?s)pub\s+struct\s+\w+\s*\{(?:[^}]*,){15,}[^}]*\}"),
            "GOD_STRUCT",
            "Struct with 15+ fields — split into smaller types",
            Severity::P1High,
        ),
        (
            mk(r"(?:\{[^{}]*){6,}"),
            "DEEP_NESTING",
            "6+ levels nesting — extract to functions",
            Severity::P1High,
        ),
        (
            mk(r"(?:static|lazy_static|thread_local)\s*!?\s*\{[^}]*Mutex"),
            "STATE_HANDLER",
            "Global mutable state — use app state injection",
            Severity::P1High,
        ),
        (
            mk(
                r"(?s)if\s+.*\{[^}]*\}\s*else\s+if\s+.*\{[^}]*\}\s*else\s+if.*\{[^}]*\}\s*else\s+if",
            ),
            "IF_CHAIN",
            "4+ if-else — use match or strategy pattern",
            Severity::P2Medium,
        ),
        (
            mk(r"pub\s+(?:async\s+)?fn\s+\w+\s*\([^)]*:\s*&(?:Vec|String|HashMap)<"),
            "CONCRETE_PARAM",
            "Concrete type in public fn — use &[T], &str, impl Trait",
            Severity::P2Medium,
        ),
        (
            mk(r"(?:\.downcast|TypeId::of|Any::type_id|is::<)"),
            "MANUAL_DISPATCH",
            "Manual type dispatch — use trait objects",
            Severity::P2Medium,
        ),
        (
            mk(r"fn\s+\w+\s*\([^)]*:\s*bool\s*[,)]"),
            "BOOL_PARAM",
            "bool parameter — use descriptive enum",
            Severity::P2Medium,
        ),
        (
            mk(r"(?s)fn\s+\w+[^{]*\{[^}]{800,}\}"),
            "LONG_METHOD",
            "Method >50 lines — extract helper functions",
            Severity::P2Medium,
        ),
    ]
}

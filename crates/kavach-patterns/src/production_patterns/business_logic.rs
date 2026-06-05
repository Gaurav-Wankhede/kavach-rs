//! Business logic anti-patterns.

use super::types::{Severity, mk};

pub(super) fn build() -> Vec<(Option<regex::Regex>, &'static str, &'static str, Severity)> {
    vec![
        (
            mk(r"(?m)^\s*(?:pub\s+)?(?:async\s+)?fn\s+\w+\s*\([^)]*\)\s*(?:->\s*\S+\s*)?\{\s*\}"),
            "EMPTY_FN",
            "Empty function body — implement business logic",
            Severity::P0Critical,
        ),
        (
            mk(r"(?:if|while|match)\s*.*(?:>=|<=|>|<|==)\s*\d{3,}"),
            "MAGIC_NUMBER",
            "Hardcoded numeric threshold — use named constant",
            Severity::P1High,
        ),
        (
            mk(r"(?:price|amount|cost|fee|total|balance|rate).*:\s*f(?:32|64)"),
            "MONEY_FLOAT",
            "Money as float — use rust_decimal::Decimal",
            Severity::P0Critical,
        ),
        (
            mk(r"\*\s*(?:100|0\.01).*%|%.*\*\s*(?:100|0\.01)"),
            "UNBOUNDED_PERCENT",
            "Percentage calc — validate 0..=100 range",
            Severity::P1High,
        ),
        (
            mk(r"(?:is_|has_|can_|should_)\w+:\s*bool.*,\s*(?:is_|has_|can_|should_)\w+:\s*bool"),
            "BOOL_FLAGS",
            "Multiple bool flags — use enum state machine",
            Severity::P2Medium,
        ),
        (
            mk(r#"==\s*"(?:active|pending|completed|failed|processing|approved|rejected)""#),
            "STRING_STATE",
            "String state comparison — use enum",
            Severity::P1High,
        ),
        (
            mk(r"/\s*\w+(?:\s*[;,\)])"),
            "DIV_NO_CHECK",
            "Division — verify divisor != 0",
            Severity::P2Medium,
        ),
        (
            mk(r"(?:qty|quantity|count|amount)\s*[<>]=?\s*-?\d"),
            "NEG_QTY",
            "Quantity comparison — ensure >= 0",
            Severity::P2Medium,
        ),
        (
            mk(r"NaiveDateTime|chrono::Utc\.timestamp\(\)"),
            "NAIVE_TIME",
            "Naive timestamp — use DateTime<Utc>",
            Severity::P1High,
        ),
        (
            mk(r"(?:f32|f64).*==|==.*(?:f32|f64)"),
            "FLOAT_EQ",
            "Float equality — use approx crate or epsilon",
            Severity::P1High,
        ),
        (
            mk(r"loop\s*\{\s*\}"),
            "INFINITE_LOOP",
            "Empty loop body — add logic or termination",
            Severity::P0Critical,
        ),
        (
            mk(r";\s*\w+\.\w+\([^)]*\)\s*;"),
            "IGNORED_RETURN",
            "Return value ignored — check result",
            Severity::P2Medium,
        ),
    ]
}

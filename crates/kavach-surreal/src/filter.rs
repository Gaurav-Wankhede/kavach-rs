// Metadata filtering for kavach memory queries
// Compiles high-level filter expressions to SurrealQL WHERE clauses
// SECURITY: All field/edge/table names are allow-listed to prevent SurrealQL injection.
// SOURCE: https://github.com/orgs/surrealdb/discussions/1330 (SurrealQL injection guidance)
// SOURCE: https://docs.rs/itertools — verified at kavach-hook/src/toon.rs:32

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt;
use surrealdb_types::SurrealValue;

/// Allow-listed field names safe to embed in `SurrealQL` queries.
const ALLOWED_FIELDS: &[&str] = &[
    "project",
    "entry_key",
    "title",
    "content",
    "status",
    "entry_status",
    "category",
    "tags",
    "decay_score",
    "access_count",
    "created_at",
    "updated_at",
    "accessed_at",
    "source",
    "spec_key",
];

/// Allow-listed graph edge names.
const ALLOWED_EDGES: &[&str] = &[
    "serves",
    "implements",
    "blocks",
    "depends_on",
    "references",
    "mentions",
    "supersedes",
    "contains",
    "modifies",
    "uses_skill",
];

/// Allow-listed target tables for graph traversal.
const ALLOWED_TABLES: &[&str] = &[
    "decision", "research", "roadmap", "pattern", "app_spec", "project", "session", "entity",
    "kanban", "part",
];

// FIX: [auth_bypass/injection] crates/kavach-surreal/src/filter.rs:3
// SYMPTOM: to_surql interpolated field/edge/table identifiers raw despite the
//          file-level claim "All field/edge/table names are allow-listed".
// WHY5: identifiers cannot be parameter-bound (only values can) — an unwired
//        allowlist is false assurance; raw interpolation is CWE-89 injection.
// ROOT_CAUSE: is_allowed_{field,edge,table}/is_safe_key had zero call sites.
// BLAST_SITE: filter.rs to_surql arms (Eq/Ne/In/NotIn/Range/Contains/
//             RelatedTo/RelatedFrom) — all now guarded fail-closed.
// RESEARCH: github.com/lfnovo/open-notebook GHSA-5wj9-f8q5-8f9c (order_by
//           identifier injection); surrealdb.com/docs/.../security-best-practices
//           ("identifiers require explicit allowlist validation in app code").
// SOLUTION: every identifier passes through these guards; a non-allowlisted
//           name collapses the clause to a never-match sentinel ("1 = 2"),
//           never reaching the query as raw text.
fn is_allowed_field(name: &str) -> bool {
    ALLOWED_FIELDS.contains(&name)
}

fn is_allowed_edge(name: &str) -> bool {
    ALLOWED_EDGES.contains(&name)
}

fn is_allowed_table(name: &str) -> bool {
    ALLOWED_TABLES.contains(&name)
}

/// `SurrealQL` clause that can never match — fail-closed sentinel returned when
/// an identifier is not on the allowlist (treats injection attempt as empty set).
const NEVER_MATCH: &str = "1 = 2";

/// Validated field identifier or the fail-closed sentinel marker (None).
fn safe_field(name: &str) -> Option<&str> {
    is_allowed_field(name).then_some(name)
}

/// Validate duration string format: ^\d+[dhmswy]$ — wired into
/// `FilterValue::RelativeDuration` `to_surql` (raw `time::now() - {dur}` was a
/// CWE-89 vector identical to the identifier case).
fn is_valid_duration(s: &str) -> bool {
    if s.is_empty() || s.len() > 16 {
        return false;
    }
    let last = match s.as_bytes().last() {
        Some(b) => *b,
        None => return false,
    };
    if !matches!(last, b'd' | b'h' | b'm' | b's' | b'w' | b'y') {
        return false;
    }
    // ASCII suffix (validated above), so byte-length minus 1 is a valid char
    // boundary; `split_at_checked` avoids both indexing_slicing and
    // arithmetic_side_effects on `s.len() - 1`.
    let split_idx = s.len().saturating_sub(1);
    let Some((digits, _)) = s.split_at_checked(split_idx) else {
        return false;
    };
    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
}

/// Validate `entry_key` for safety (alphanumeric + `.`, `_`, `-`).
fn is_safe_key(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// Filter expression AST for metadata-aware queries
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, SurrealValue)]
#[non_exhaustive]
pub enum FilterExpr {
    Eq {
        field: String,
        value: FilterValue,
    },
    Ne {
        field: String,
        value: FilterValue,
    },
    In {
        field: String,
        values: Vec<FilterValue>,
    },
    NotIn {
        field: String,
        values: Vec<FilterValue>,
    },
    Range {
        field: String,
        gte: Option<FilterValue>,
        lte: Option<FilterValue>,
    },
    Contains {
        field: String,
        substring: String,
    },
    And(Vec<Self>),
    Or(Vec<Self>),
    Not(Box<Self>),
    RelatedTo {
        edge: String,
        target_table: String,
        target_key: String,
    },
    RelatedFrom {
        edge: String,
        source_table: String,
        source_key: String,
    },
}

/// Typed filter values with proper `SurrealQL` escaping
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, SurrealValue)]
#[non_exhaustive]
pub enum FilterValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    DateTime(chrono::DateTime<chrono::Utc>),
    RelativeDuration(String),
}

impl FilterValue {
    #[must_use]
    pub fn to_surql(&self) -> String {
        match self {
            Self::String(s) => format!("'{}'", s.replace('\'', "\\'")),
            Self::Int(n) => n.to_string(),
            Self::Float(f) => f.to_string(),
            Self::Bool(b) => b.to_string(),
            Self::DateTime(dt) => format!("d'{}'", dt.to_rfc3339()),
            Self::RelativeDuration(dur) if is_valid_duration(dur) => {
                format!("time::now() - {dur}")
            }
            // Invalid duration → epoch sentinel: a valid expression that
            // selects nothing recent, never raw injected text.
            Self::RelativeDuration(_) => "d'1970-01-01T00:00:00Z'".to_owned(),
        }
    }
}

impl FilterExpr {
    #[must_use]
    pub fn to_surql(&self) -> String {
        match self {
            Self::Eq { field, value } => safe_field(field).map_or_else(
                || NEVER_MATCH.to_owned(),
                |f| format!("{f} = {}", value.to_surql()),
            ),
            Self::Ne { field, value } => safe_field(field).map_or_else(
                || NEVER_MATCH.to_owned(),
                |f| format!("{f} != {}", value.to_surql()),
            ),
            Self::In { field, values } => safe_field(field).map_or_else(
                || NEVER_MATCH.to_owned(),
                |f| {
                    let vals = values.iter().map(FilterValue::to_surql).join(", ");
                    format!("{f} IN [{vals}]")
                },
            ),
            Self::NotIn { field, values } => safe_field(field).map_or_else(
                || NEVER_MATCH.to_owned(),
                |f| {
                    let vals = values.iter().map(FilterValue::to_surql).join(", ");
                    format!("{f} NOT IN [{vals}]")
                },
            ),
            Self::Range { field, gte, lte } => {
                let Some(f) = safe_field(field) else {
                    return NEVER_MATCH.to_owned();
                };
                let mut clauses = vec![];
                if let Some(v) = gte {
                    clauses.push(format!("{f} >= {}", v.to_surql()));
                }
                if let Some(v) = lte {
                    clauses.push(format!("{f} <= {}", v.to_surql()));
                }
                if clauses.is_empty() {
                    "true".to_owned()
                } else {
                    clauses.join(" AND ")
                }
            }
            Self::Contains { field, substring } => safe_field(field).map_or_else(
                || NEVER_MATCH.to_owned(),
                |f| format!("{f} CONTAINS '{}'", substring.replace('\'', "\\'")),
            ),
            Self::And(exprs) if exprs.is_empty() => "true".to_owned(),
            Self::And(exprs) => {
                let parts: Vec<_> = exprs.iter().map(Self::to_surql).collect();
                format!("({})", parts.join(" AND "))
            }
            Self::Or(exprs) if exprs.is_empty() => "false".to_owned(),
            Self::Or(exprs) => {
                let parts: Vec<_> = exprs.iter().map(Self::to_surql).collect();
                format!("({})", parts.join(" OR "))
            }
            Self::Not(expr) => format!("NOT ({})", expr.to_surql()),
            Self::RelatedTo {
                edge,
                target_table,
                target_key,
            } => {
                if !is_allowed_edge(edge)
                    || !is_allowed_table(target_table)
                    || !is_safe_key(target_key)
                {
                    return NEVER_MATCH.to_owned();
                }
                format!(
                    "->{edge}->({target_table} WHERE entry_key = '{}')",
                    target_key.replace('\'', "\\'")
                )
            }
            Self::RelatedFrom {
                edge,
                source_table,
                source_key,
            } => {
                if !is_allowed_edge(edge)
                    || !is_allowed_table(source_table)
                    || !is_safe_key(source_key)
                {
                    return NEVER_MATCH.to_owned();
                }
                format!(
                    "<-{edge}<-({source_table} WHERE entry_key = '{}')",
                    source_key.replace('\'', "\\'")
                )
            }
        }
    }

    pub fn and(exprs: impl IntoIterator<Item = Self>) -> Self {
        Self::And(exprs.into_iter().collect())
    }

    pub fn or(exprs: impl IntoIterator<Item = Self>) -> Self {
        Self::Or(exprs.into_iter().collect())
    }

    #[must_use]
    pub fn not(self) -> Self {
        Self::Not(Box::new(self))
    }
}

impl fmt::Display for FilterExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_surql())
    }
}

/// Builder for constructing filter expressions fluently
#[derive(Debug, Default)]
pub struct FilterBuilder {
    expressions: Vec<FilterExpr>,
}

impl FilterBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn eq(mut self, field: impl Into<String>, value: impl Into<FilterValue>) -> Self {
        self.expressions.push(FilterExpr::Eq {
            field: field.into(),
            value: value.into(),
        });
        self
    }

    #[must_use]
    pub fn ne(mut self, field: impl Into<String>, value: impl Into<FilterValue>) -> Self {
        self.expressions.push(FilterExpr::Ne {
            field: field.into(),
            value: value.into(),
        });
        self
    }

    #[must_use]
    pub fn in_set(
        mut self,
        field: impl Into<String>,
        values: impl IntoIterator<Item = impl Into<FilterValue>>,
    ) -> Self {
        self.expressions.push(FilterExpr::In {
            field: field.into(),
            values: values.into_iter().map(Into::into).collect(),
        });
        self
    }

    #[must_use]
    pub fn not_in_set(
        mut self,
        field: impl Into<String>,
        values: impl IntoIterator<Item = impl Into<FilterValue>>,
    ) -> Self {
        self.expressions.push(FilterExpr::NotIn {
            field: field.into(),
            values: values.into_iter().map(Into::into).collect(),
        });
        self
    }

    #[must_use]
    pub fn range(
        mut self,
        field: impl Into<String>,
        gte: Option<impl Into<FilterValue>>,
        lte: Option<impl Into<FilterValue>>,
    ) -> Self {
        self.expressions.push(FilterExpr::Range {
            field: field.into(),
            gte: gte.map(Into::into),
            lte: lte.map(Into::into),
        });
        self
    }

    #[must_use]
    pub fn since(mut self, field: impl Into<String>, duration: impl Into<String>) -> Self {
        self.expressions.push(FilterExpr::Range {
            field: field.into(),
            gte: Some(FilterValue::RelativeDuration(duration.into())),
            lte: None,
        });
        self
    }

    #[must_use]
    pub fn contains(mut self, field: impl Into<String>, substring: impl Into<String>) -> Self {
        self.expressions.push(FilterExpr::Contains {
            field: field.into(),
            substring: substring.into(),
        });
        self
    }

    #[must_use]
    pub fn related_to(
        mut self,
        edge: impl Into<String>,
        target_table: impl Into<String>,
        target_key: impl Into<String>,
    ) -> Self {
        self.expressions.push(FilterExpr::RelatedTo {
            edge: edge.into(),
            target_table: target_table.into(),
            target_key: target_key.into(),
        });
        self
    }

    #[must_use]
    pub fn build(self) -> Option<FilterExpr> {
        match self.expressions.len() {
            0 => None,
            1 => self.expressions.into_iter().next(),
            _ => Some(FilterExpr::And(self.expressions)),
        }
    }
}

impl From<&str> for FilterValue {
    fn from(s: &str) -> Self {
        Self::String(s.to_owned())
    }
}

impl From<String> for FilterValue {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<i64> for FilterValue {
    fn from(n: i64) -> Self {
        Self::Int(n)
    }
}

impl From<i32> for FilterValue {
    fn from(n: i32) -> Self {
        Self::Int(i64::from(n))
    }
}

impl From<f64> for FilterValue {
    fn from(f: f64) -> Self {
        Self::Float(f)
    }
}

impl From<bool> for FilterValue {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<chrono::DateTime<chrono::Utc>> for FilterValue {
    fn from(dt: chrono::DateTime<chrono::Utc>) -> Self {
        Self::DateTime(dt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eq_filter() {
        let filter = FilterExpr::Eq {
            field: "entry_status".to_owned(),
            value: FilterValue::String("verified".to_owned()),
        };
        assert_eq!(filter.to_surql(), "entry_status = 'verified'");
    }

    #[test]
    fn test_in_filter() {
        let filter = FilterExpr::In {
            field: "category".to_owned(),
            values: vec![
                FilterValue::String("arch".to_owned()),
                FilterValue::String("spec".to_owned()),
            ],
        };
        assert_eq!(filter.to_surql(), "category IN ['arch', 'spec']");
    }

    #[test]
    fn test_range_filter() {
        let filter = FilterExpr::Range {
            field: "created_at".to_owned(),
            gte: Some(FilterValue::RelativeDuration("30d".to_owned())),
            lte: None,
        };
        assert_eq!(filter.to_surql(), "created_at >= time::now() - 30d");
    }

    #[test]
    fn test_and_filter() {
        let filter = FilterExpr::And(vec![
            FilterExpr::Eq {
                field: "entry_status".to_owned(),
                value: FilterValue::String("verified".to_owned()),
            },
            FilterExpr::In {
                field: "category".to_owned(),
                values: vec![FilterValue::String("arch".to_owned())],
            },
        ]);
        assert_eq!(
            filter.to_surql(),
            "(entry_status = 'verified' AND category IN ['arch'])"
        );
    }

    #[test]
    fn test_related_to_filter() {
        let filter = FilterExpr::RelatedTo {
            edge: "serves".to_owned(),
            target_table: "roadmap".to_owned(),
            target_key: "payment-flow".to_owned(),
        };
        assert_eq!(
            filter.to_surql(),
            "->serves->(roadmap WHERE entry_key = 'payment-flow')"
        );
    }

    #[test]
    fn test_builder() {
        let Some(filter) = FilterBuilder::new()
            .eq("entry_status", "verified")
            .in_set("category", ["arch", "spec"])
            .since("created_at", "30d")
            .build()
        else {
            panic!("builder returned None for non-empty expressions");
        };

        let surql = filter.to_surql();
        assert!(surql.contains("entry_status = 'verified'"));
        assert!(surql.contains("category IN ['arch', 'spec']"));
        assert!(surql.contains("created_at >= time::now() - 30d"));
    }

    #[test]
    fn injection_field_name_fails_closed() {
        // CWE-89: a non-allowlisted field name must NOT reach the query raw.
        let evil = FilterExpr::Eq {
            field: "1=1 OR title".to_owned(),
            value: FilterValue::String("x".to_owned()),
        };
        assert_eq!(evil.to_surql(), "1 = 2");
        // Allowlisted field still works.
        let ok = FilterExpr::Eq {
            field: "entry_status".to_owned(),
            value: FilterValue::String("verified".to_owned()),
        };
        assert_eq!(ok.to_surql(), "entry_status = 'verified'");
    }

    #[test]
    fn injection_edge_table_fails_closed() {
        let evil = FilterExpr::RelatedTo {
            edge: "serves; DELETE roadmap".to_owned(),
            target_table: "roadmap".to_owned(),
            target_key: "k".to_owned(),
        };
        assert_eq!(evil.to_surql(), "1 = 2");
        let evil_tbl = FilterExpr::RelatedTo {
            edge: "serves".to_owned(),
            target_table: "roadmap WHERE 1=1".to_owned(),
            target_key: "k".to_owned(),
        };
        assert_eq!(evil_tbl.to_surql(), "1 = 2");
    }

    #[test]
    fn injection_duration_fails_closed() {
        let evil = FilterExpr::Range {
            field: "created_at".to_owned(),
            gte: Some(FilterValue::RelativeDuration("30d; DROP".to_owned())),
            lte: None,
        };
        // Invalid duration collapses to epoch sentinel, never raw text.
        assert!(evil.to_surql().contains("1970-01-01"));
        assert!(!evil.to_surql().contains("DROP"));
    }

    #[test]
    fn test_string_escape() {
        let filter = FilterExpr::Eq {
            field: "title".to_owned(),
            value: FilterValue::String("it's a test".to_owned()),
        };
        assert_eq!(filter.to_surql(), "title = 'it\\'s a test'");
    }
}

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt;
use surrealdb_types::SurrealValue;

use super::guard::{is_allowed_edge, is_allowed_table, is_safe_key, safe_field, NEVER_MATCH};
use super::value::FilterValue;

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

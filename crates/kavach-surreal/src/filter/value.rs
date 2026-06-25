use serde::{Deserialize, Serialize};
use surrealdb_types::SurrealValue;

use super::guard::is_valid_duration;

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
            Self::RelativeDuration(_) => "d'1970-01-01T00:00:00Z'".to_owned(),
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

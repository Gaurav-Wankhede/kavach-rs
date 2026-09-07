use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "closed relation taxonomy matched exhaustively in as_str/from_str"
)]
pub enum RelationType {
    Contains,
    DependsOn,
    Modifies,
    References,
    Mentions,
    WorksOn,
    Owns,
}

impl RelationType {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Contains => "contains",
            Self::DependsOn => "depends_on",
            Self::Modifies => "modifies",
            Self::References => "references",
            Self::Mentions => "mentions",
            Self::WorksOn => "works_on",
            Self::Owns => "owns",
        }
    }

    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "contains" => Some(Self::Contains),
            "depends_on" => Some(Self::DependsOn),
            "modifies" => Some(Self::Modifies),
            "references" => Some(Self::References),
            "mentions" => Some(Self::Mentions),
            "works_on" => Some(Self::WorksOn),
            "owns" => Some(Self::Owns),
            _unknown => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Entity {
    pub id: Option<String>,
    pub entity_type: String,
    pub name: String,
    pub properties: Option<serde_json::Value>,
    pub content_hash: Option<String>,
    pub project: Option<String>,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Edge {
    pub id: Option<String>,
    pub r#in: String,
    pub out: String,
    pub weight: f64,
    pub properties: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate struct-literal DTO (kavach-rpc); non_exhaustive => E0639"
)]
pub struct RelateParams {
    pub from_id: String,
    pub to_id: String,
    pub rel_type: RelationType,
    pub weight: Option<f64>,
    pub properties: Option<serde_json::Value>,
}

use serde::{Deserialize, Serialize};

/// Mermaid node shape for a flow step. `Rect` is the default (`["label"]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum NodeShape {
    /// `["label"]`
    #[default]
    Rect,
    /// `("label")`
    Round,
    /// `(["label"])`
    Stadium,
    /// `{"label"}`
    Diamond,
    /// `(("label"))`
    Circle,
}

impl NodeShape {
    /// Parse a shape hint; unknown values fall back to `Rect` (lenient ingest).
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s {
            "round" => Self::Round,
            "stadium" => Self::Stadium,
            "diamond" => Self::Diamond,
            "circle" => Self::Circle,
            _ => Self::Rect,
        }
    }

    /// The `snake_case` token persisted in `properties.shape`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rect => "rect",
            Self::Round => "round",
            Self::Stadium => "stadium",
            Self::Diamond => "diamond",
            Self::Circle => "circle",
        }
    }

    /// Wrap a (Mermaid-escaped) label in this shape's delimiters.
    pub(super) fn wrap(self, label: &str) -> String {
        match self {
            Self::Rect => format!("[\"{label}\"]"),
            Self::Round => format!("(\"{label}\")"),
            Self::Stadium => format!("([\"{label}\"])"),
            Self::Diamond => format!("{{\"{label}\"}}"),
            Self::Circle => format!("((\"{label}\"))"),
        }
    }
}

/// Mermaid node ids reject hyphens / quotes / spaces — replace them, mirroring
/// the ER-diagram emitter (`cmd/db/pg/er.rs::sanitize`). A non-alphanumeric,
/// non-`_` char becomes `_` so the id is always a valid Mermaid identifier.
pub(super) fn sanitize_id(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

/// Escape a label for inside a Mermaid quoted string: `"` would close the
/// string and `\n`/`\r` would break the line, so neutralize them.
pub(super) fn escape_label(label: &str) -> String {
    label.replace('"', "&quot;").replace(['\n', '\r'], " ")
}

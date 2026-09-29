//! A resolved declaration site for a symbol, ranked config-origin first.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Kind {
    EnvVar,
    ConfigField,
    Schema,
    Const,
    Static,
    Default,
    Type,
    Function,
    DocAnchor,
    Param,
    Variant,
    LetBinding,
}

impl Kind {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::EnvVar => "env-var",
            Self::ConfigField => "config-field",
            Self::Schema => "schema",
            Self::Const => "const",
            Self::Static => "static",
            Self::Default => "default-impl",
            Self::Type => "type",
            Self::Function => "fn",
            Self::DocAnchor => "doc-anchor",
            Self::Param => "param",
            Self::Variant => "enum-variant",
            Self::LetBinding => "let",
        }
    }

    pub(super) const fn is_centralized(self) -> bool {
        matches!(
            self,
            Self::EnvVar | Self::ConfigField | Self::Schema | Self::Const | Self::Default
        )
    }

    pub(super) fn from_label(label: &str) -> Option<Self> {
        match label {
            "env-var" => Some(Self::EnvVar),
            "config-field" => Some(Self::ConfigField),
            "schema" => Some(Self::Schema),
            "const" => Some(Self::Const),
            "static" => Some(Self::Static),
            "default-impl" => Some(Self::Default),
            "type" => Some(Self::Type),
            "fn" => Some(Self::Function),
            "doc-anchor" => Some(Self::DocAnchor),
            "param" => Some(Self::Param),
            "enum-variant" => Some(Self::Variant),
            "let" => Some(Self::LetBinding),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct Site {
    pub kind: Kind,
    pub file: String,
    pub line: usize,
}

impl Site {
    pub(super) fn dedup_key(&self) -> String {
        format!("{}|{}|{:?}", self.file, self.line, self.kind)
    }
}

//! A resolved declaration site for a symbol, ranked config-origin first.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Kind {
    EnvVar,
    ConfigField,
    Const,
    Static,
    Default,
    Type,
    Function,
    Param,
    Variant,
    LetBinding,
}

impl Kind {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::EnvVar => "env-var",
            Self::ConfigField => "config-field",
            Self::Const => "const",
            Self::Static => "static",
            Self::Default => "default-impl",
            Self::Type => "type",
            Self::Function => "fn",
            Self::LetBinding => "let",
        }
    }

    pub(super) const fn is_centralized(self) -> bool {
        matches!(self, Self::EnvVar | Self::ConfigField | Self::Const | Self::Default)
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

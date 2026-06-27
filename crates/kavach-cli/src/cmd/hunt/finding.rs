//! Unified finding emitted by every detector the hunter runs.

/// Worst-practice severity, mapped from the source detector's tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Block,
    Warn,
    Advisory,
}

impl Severity {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Block => "BLOCK",
            Self::Warn => "WARN",
            Self::Advisory => "ADVISORY",
        }
    }
}

/// One detector hit at a precise site — the antivirus "signature match".
#[derive(Debug, Clone)]
pub struct Finding {
    pub detector: &'static str,
    pub file: String,
    pub line: usize,
    pub severity: Severity,
    pub category: String,
    pub snippet: String,
    pub fix: String,
}

impl Finding {
    #[must_use]
    pub fn dedup_key(&self) -> String {
        format!("{}|{}|{}", self.detector, self.file, self.line)
    }
}

//! Unified finding + severity + lens taxonomy for the consolidated `kavach audit`.

/// Which audit lens produced a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lens {
    Yagni,
    SilentFail,
    WorstPractice,
    Security,
}

impl Lens {
    pub(crate) const fn slug(self) -> &'static str {
        match self {
            Self::Yagni => "yagni",
            Self::SilentFail => "silent-fail",
            Self::WorstPractice => "worst-practice",
            Self::Security => "security",
        }
    }
}

/// Severity tier shared by every lens (was 2 separate enums pre-merge).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Severity {
    Block,
    Warn,
    Advisory,
}

impl Severity {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Block => "BLOCK",
            Self::Warn => "WARN",
            Self::Advisory => "ADVISORY",
        }
    }
}

/// One unified finding from any lens at a precise site.
#[derive(Debug, Clone)]
pub(crate) struct Finding {
    pub lens: Lens,
    pub detector: String,
    pub file: String,
    pub line: usize,
    pub severity: Severity,
    pub hint: String,
    pub fix: String,
}

impl Finding {
    pub(crate) fn dedup_key(&self) -> String {
        format!("{}|{}|{}|{}", self.lens.slug(), self.detector, self.file, self.line)
    }
}

#[cfg(test)]
#[path = "finding_test.rs"]
mod finding_test;

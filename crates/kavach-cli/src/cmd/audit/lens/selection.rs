//! Lens selection: which audit lens(es) a run executes, parsed from `--lens`.

/// Which lenses to run. `All` runs every lens (default).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Selection {
    Code,
    SelfAudit,
    Security,
    All,
}

impl Selection {
    /// Map the `--lens` flag to a selection; unknown/empty defaults to `All`.
    #[must_use]
    pub(crate) fn from_flag(flag: &str) -> Self {
        match flag {
            "code" => Self::Code,
            "self" => Self::SelfAudit,
            "security" => Self::Security,
            _ => Self::All,
        }
    }
}

#[cfg(test)]
#[path = "selection_test.rs"]
mod selection_test;

use std::sync::LazyLock;

pub(super) struct Signal {
    pub(super) positive: LazyLock<Result<regex::Regex, regex::Error>>,
    pub(super) negation: LazyLock<Result<regex::Regex, regex::Error>>,
}

impl Signal {
    pub(super) fn fires(&self, msg: &str) -> Result<bool, regex::Error> {
        let pos = self.positive.as_ref().map_err(Clone::clone)?;
        if !pos.is_match(msg) {
            return Ok(false);
        }
        let neg = self.negation.as_ref().map_err(Clone::clone)?;
        Ok(!neg.is_match(msg))
    }
}

pub(super) const NEVER: &str = r"\bzzzz_unsatisfiable_sentinel_never_matches_zzzz\b";

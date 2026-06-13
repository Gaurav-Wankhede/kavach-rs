//! Advisory-scope guard — the structural C2 boundary. The learned controller
//! tunes ONLY advisory gates; hard P0/forbid gates bypass it entirely. Encoding
//! scope as a VALUE the constructor checks (not a doc comment) means a hard-block
//! path can neither type-check nor value-check its way into policy selection.
use super::value::ActionValue;

/// The gate scope a candidate set originates from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GateScope {
    /// An advisory gate the controller MAY tune.
    Advisory,
    /// A hard P0/forbid gate the controller must NEVER touch.
    HardBlock,
}

/// Candidates proven to come from an advisory gate.
///
/// The only input [`super::choose`] accepts. A hard-block decision path cannot
/// reach the learned controller: the constructor REFUSES [`GateScope::HardBlock`]
/// (fail-closed), so a P0 gate can neither type-check nor value-check its way
/// into selection.
#[derive(Debug, Clone, Copy)]
pub struct AdvisoryCandidates<'a>(&'a [ActionValue]);

impl<'a> AdvisoryCandidates<'a> {
    /// Wrap `candidates` IFF they originate from an advisory gate; `None` for
    /// [`GateScope::HardBlock`]. The structural C2 boundary, exercised by a test
    /// rather than asserted in prose.
    #[must_use]
    pub const fn new(candidates: &'a [ActionValue], scope: GateScope) -> Option<Self> {
        match scope {
            GateScope::Advisory => Some(Self(candidates)),
            GateScope::HardBlock => None,
        }
    }

    /// The wrapped advisory candidates.
    #[must_use]
    pub const fn as_slice(&self) -> &[ActionValue] {
        self.0
    }
}

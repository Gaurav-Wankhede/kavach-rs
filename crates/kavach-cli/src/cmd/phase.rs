// ARCH: PhaseCommandHandler — CLI handlers for SDLC phase management
// PATTERN: phase_gate | SCOPE: cli | CAP: AP | SEARCHED: 2026-04

mod iteration;
mod spike;
mod status;
mod tier;

use crate::cli::PhaseAction;
pub(crate) use iteration::{handle_iteration_done, handle_iteration_list, handle_iteration_start};
pub(crate) use spike::{handle_spike_end, handle_spike_start};
pub(crate) use status::{handle_advance, handle_set, handle_status};
pub(crate) use tier::handle_tier_set;

/// `kavach phase <action>` — manage SDLC development phases.
pub(super) fn run(action: PhaseAction) -> i32 {
    match action {
        PhaseAction::Status => handle_status(),
        PhaseAction::Advance => handle_advance(),
        PhaseAction::Set { phase } => handle_set(&phase),
        PhaseAction::IterationStart { file } => handle_iteration_start(file.as_deref()),
        PhaseAction::IterationDone => handle_iteration_done(),
        PhaseAction::IterationList => handle_iteration_list(),
        PhaseAction::TierSet {
            tier,
            project,
            reason,
            override_flag,
        } => handle_tier_set(&tier, &project, &reason, override_flag),
        PhaseAction::SpikeStart {
            project,
            hours,
            reason,
        } => handle_spike_start(&project, hours, &reason),
        PhaseAction::SpikeEnd { project } => handle_spike_end(&project),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_accepts_all_phase_action_variants() {
        let _: fn(PhaseAction) -> i32 = run;
    }
}

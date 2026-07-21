// ARCH: see kavach db get --category decision --key arch.decision.fourteen_prefix_const_table

pub mod gate_config;
pub use gate_config::{GateValueDto, gate_enabled, gate_patterns, gate_text, gate_threshold};

    ArtifactValidator, AutoDraftSource, FOURTEEN_PREFIXES, MissingPrefix, MissingReason,
    ProjectTier, RequiredPrefix, SpecCategory, SpikeMode, WitnessResult,
};

pub mod priority;
pub use priority::Priority;

pub mod effort_input;
pub use effort_input::EffortInput;

pub mod hook_io;
pub use hook_io::{HookInput, HookResponse, HookSpecificOutput};

pub mod memory_status;
pub use memory_status::MemoryStatus;

// TIME: O(0) runtime — expands to () | SPACE: O(0)
// YEAR: 2026 | SEARCHED: 2026-05

/// Zero-cost marker macro. The kavach binary scans source for these
/// invocations and syncs them to kanban as roadmap entries keyed by <file:line>.
///
/// At runtime this expands to a no-op — no impact on user binary.
#[macro_export]
macro_rules! kavach_todo {
    ($desc:literal $(,)?) => {{
        let _ = $desc;
    }};
    ($desc:literal, $($rest:tt)*) => {{
        let _ = $desc;
    }};
}

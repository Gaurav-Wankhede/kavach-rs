//! Skill template generator from detected code patterns.
//! Auto-creates TOON skill files based on project analysis.

pub mod detector;
pub mod emitter;
pub mod patterns;
pub mod template;

pub use detector::{DetectedPattern, PatternType, detect_patterns};
pub use emitter::emit_skill;
pub use patterns::FrameworkPattern;
pub use template::generate_skill;

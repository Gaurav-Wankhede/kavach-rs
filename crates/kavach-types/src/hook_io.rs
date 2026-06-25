mod input;
mod output;
#[cfg(test)]
#[path = "hook_io/tests.rs"]
mod tests;

pub use input::HookInput;
pub use output::{HookResponse, HookSpecificOutput};

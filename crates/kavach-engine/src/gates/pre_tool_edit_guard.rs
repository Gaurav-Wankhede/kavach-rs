use kavach_types::HookInput;

// SOURCE: https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html
pub(crate) fn check_edit_staleness(_input: &HookInput) -> Option<String> {
    None
}

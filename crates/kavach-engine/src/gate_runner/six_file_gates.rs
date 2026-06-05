//! Dispatch for the six-file-context gate family.
use kavach_types::HookInput;

use crate::error::EngineError;
use crate::gates;

/// Match a six-file-context gate. Returns `None` to fall through.
pub(super) fn dispatch(gate_name: &str, input: &HookInput) -> Option<Result<(), EngineError>> {
    let result = match gate_name {
        "six-file-intent" => gates::six_file::intent::run(input),
        "six-file-pre-write" => gates::six_file::pre_write_path::run(input),
        "pre-implementation" => gates::six_file::pre_implementation::run(input),
        "post-implementation" => gates::six_file::post_implementation::run(input),
        _ => return None,
    };
    Some(result)
}

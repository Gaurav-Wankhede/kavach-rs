//! `post_tool_bash` tests, split by family: output classifiers vs scope/clear.
#[path = "post_tool_bash/tests/capture.rs"]
mod capture;
#[path = "post_tool_bash/tests/classify.rs"]
mod classify;
#[path = "post_tool_bash/tests/scope.rs"]
mod scope;
#[path = "post_tool_bash/tests/tdd_compile_fail_test.rs"]
mod tdd_compile_fail_test;
#[path = "post_tool_bash/tests/tdd_nested_path_test.rs"]
mod tdd_nested_path_test;
#[path = "post_tool_bash/tests/tdd_integration_test.rs"]
mod tdd_integration_test;
#[path = "post_tool_bash/tests/tests_track_test.rs"]
mod tests_track_test;

//! P0-block + P1-advisory rendering coverage.
use crate::gates::pre_write_ts_guard::{check, format_advisory};

#[test]
fn should_block_when_as_any_cast() {
    assert!(check("src/App.tsx", "const x = y as any;").is_some());
}

#[test]
fn should_block_when_console_log() {
    assert!(check("src/App.tsx", "console.log('debug');").is_some());
}

#[test]
fn should_allow_when_typed_fetch() {
    assert!(check("src/App.tsx", "const user: User = await fetchUser();").is_none());
}

#[test]
fn should_skip_test_files() {
    assert!(check("src/App.test.tsx", "const x = y as any;").is_none());
}

#[test]
fn should_skip_config_files() {
    assert!(check("vite.config.ts", "eval(x)").is_none());
}

#[test]
fn should_emit_no_advisory_for_clean_code() {
    assert!(format_advisory("src/App.tsx", "const x = 1;").is_none());
}

//! Toolbelt resolution-contract + cache-consistency tests.
use super::cache::which;
use super::search::search;
use super::tool::Tool;

// NOTE: rg/fd availability is environment-dependent (CI runners often lack
// them). These tests assert the resolution CONTRACT — rust tool when present,
// documented fallback when absent — never the physical presence of a binary,
// which would make the suite fail on any machine without the toolbelt.
#[test]
fn rg_resolves_to_rust_tool_or_grep_fallback() {
    let resolved = Tool::Rg.resolve();
    if Tool::Rg.is_available() {
        assert_eq!(resolved, "rg");
    } else {
        assert_eq!(resolved, "grep");
    }
}

#[test]
fn fd_resolves_to_rust_tool_or_find_fallback() {
    let resolved = Tool::Fd.resolve();
    if Tool::Fd.is_available() {
        assert_eq!(resolved, "fd");
    } else {
        assert_eq!(resolved, "find");
    }
}

#[test]
fn resolve_uses_rust_tool_when_available() {
    let tool = Tool::Rg;
    if tool.is_available() {
        assert_eq!(tool.resolve(), "rg");
    } else {
        assert_eq!(tool.resolve(), "grep");
    }
}

#[test]
fn search_returns_output_or_not_found_when_rg_absent() {
    // search() fails closed with NotFound when rg is not installed; when it
    // IS installed it must return Ok. Both are correct — assert per-env.
    match search("fn ", ".") {
        Ok(_) => assert!(Tool::Rg.is_available(), "Ok implies rg present"),
        Err(e) => {
            assert!(!Tool::Rg.is_available(), "Err only when rg absent");
            assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
        }
    }
}

#[test]
fn cache_returns_consistent_result_across_calls() {
    let first = which("rg");
    let second = which("rg");
    let third = which("rg");
    assert_eq!(first, second);
    assert_eq!(second, third);
}

#[test]
fn cache_handles_missing_program() {
    let result = which("definitely_not_a_real_program_xyz_42");
    assert!(!result, "missing program should return false");
    let cached = which("definitely_not_a_real_program_xyz_42");
    assert!(!cached, "cached missing program should still return false");
}

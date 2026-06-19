//! kavach-patterns — write-time + bash-time pattern detection library.
//!
//! Every guard module exposes a `detect()`/`inspect()` returning [`severity::Violation`]s
//! tagged with a [`severity::Severity`] tier. The host crate (`kavach-engine`) maps
//! tier → hook action per `kavach-engine/CLAUDE.md` "Gate Severity Policy" + the
//! wiring map there. Library code never calls `exit_*` itself — gates stay pure and
//! testable. Adding a module: implement detector → wire in `kavach-engine/src/gates/`
//! → add a regression test exercising the engine entry point → update the wiring map.

mod checks;
mod config;
pub mod dedup_guard;
mod detect;
mod file_types;
mod regex_patterns;
pub mod rust_guard;
pub(crate) mod rust_patterns;
pub mod severity;
pub mod sql_guard;
pub(crate) mod sql_patterns;
pub mod ts_guard;
pub(crate) mod ts_patterns;

pub use checks::{
    classify_intent, is_blocked, is_code_file, is_infra_file, is_large_file, is_sensitive,
    is_valid_agent, sanitize_path, validate_identifier,
};
pub use config::{AntiProdLevel, AntiProdResult, Config, load, reload};
pub use detect::{detect_antiprod, detect_mock_data};
pub use file_types::{
    is_api_client_file, is_astro_file, is_backend_file, is_dockerfile, is_frontend_file,
    is_go_file, is_handler_file, is_java_file, is_non_config_file, is_python_file, is_rust_file,
    is_shell_file, is_test_file,
};

pub mod a11y_guard;
pub mod algo_complexity_guard;
pub mod alloc_guard;
pub mod arch_guard;
pub mod banned_css_guard;
pub mod comment_noise_guard;
pub mod complexity_guard;
pub mod crypto_guard;
pub mod db_security_guard;
pub mod frontend_security_guard;
pub mod gnap_guard;
pub mod k_pri;
pub mod legacy_tool_guard;
pub mod loophole_lens;
pub mod owasp_guard;
pub mod secrecy_guard;
pub mod silent_io_guard;
pub mod ux_guard;

pub mod algo_selection;
pub mod api_gateway;
pub mod api_management_guard;
pub mod async_sync_guard;
pub mod atomic_ui_guard;
pub mod axum_guard;
pub mod bandit_log;
pub mod bidi_unicode_guard;
pub mod database_ops_guard;
pub mod design_patterns_guard;
pub mod design_patterns_rules;
pub mod design_patterns_scan;
pub mod destructive_cli_guard;
pub mod dioxus_guard;
pub mod dsa_guard;
pub mod eval_replay;
pub mod finops_guard;
pub mod irreversible_guard;
pub mod laziness_guard;
pub mod micro_file_guard;
pub mod migration_safety_guard;
pub mod observability_guard;
pub mod pii_data_guard;
pub mod production_patterns;
pub mod prompt_injection_guard;
pub mod reflect;
pub mod reward;
pub mod reward_ledger;
pub mod rust_196_guard;
pub mod rust_lint_guard;
pub mod security_scanner;
pub mod shallow_verdict_guard;
pub mod skill_keyword_router;
pub mod skill_manifest;
pub mod skill_precision;
pub mod solid_guard;
pub mod system_design_guard;
pub mod tool_chain_validator;
pub mod trust_score;
pub mod unpersisted_decision_guard;
pub mod webhook_signature_guard;

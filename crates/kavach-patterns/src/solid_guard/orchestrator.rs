//! SOLID check orchestrator — calls all check modules.

use super::{dip_checks, isp_checks, lsp_checks, ocp_checks, other_checks, srp_checks};
use crate::solid_guard::SolidViolation;

pub(super) fn run_all(
    p: &[regex::Regex],
    file_path: &str,
    content: &str,
    v: &mut Vec<SolidViolation>,
) {
    srp_checks::check_god_struct(p, content, v);
    srp_checks::check_long_async_fn(p, content, v);
    srp_checks::check_conflated_derives(p, content, v);
    srp_checks::check_handler_builds_router(p, content, v);
    ocp_checks::check_provider_match(p, content, v);
    ocp_checks::check_string_dispatch(p, content, v);
    ocp_checks::check_policy_with_vendor_switch(p, content, v);
    lsp_checks::check_panic_in_trait_impl(p, content, v);
    lsp_checks::check_result_then_unwrap(p, content, v);
    lsp_checks::check_block_on_in_trait_impl(p, content, v);
    lsp_checks::check_axum_lsp_service_panic(p, content, v);
    isp_checks::check_fat_trait(p, content, v);
    isp_checks::check_storage_god_trait(p, content, v);
    isp_checks::check_catchall_method(p, content, v);
    dip_checks::check_concrete_client_param(p, content, v);
    dip_checks::check_concrete_service_field(p, content, v);
    dip_checks::check_axum_state_concrete(p, content, v);
    dip_checks::check_axum_extension_concrete(p, content, v);
    dip_checks::check_domain_imports_infra(p, file_path, content, v);
    dip_checks::check_lazy_global_client(p, content, v);
    other_checks::check_handler_raw_request(p, content, v);
}

use super::backend::check_backend_boundary;
use super::backend_flags::BackendFlags;
use super::boundary::{Boundary, classify_boundary};
use super::cross_boundary::check_cross_boundary;
use super::database::check_database_boundary;
use super::flags::Flags;
use super::frontend::check_frontend_boundary;
use super::gateway::check_gateway_boundary;
use super::patterns::is_api_relevant;
use super::types::ApiViolation;
use super::webhook::check_webhook_boundary;

#[must_use]
pub fn detect(file_path: &str, content: &str) -> Vec<ApiViolation> {
    if !is_api_relevant(file_path) {
        return vec![];
    }
    if crate::file_types::is_test_file(file_path) {
        return vec![];
    }

    let mut v = Vec::new();
    let boundary = classify_boundary(file_path, content);
    let flags = Flags::detect(content);

    match boundary {
        Boundary::FrontendComponent => check_frontend_boundary(content, &mut v, flags.authfetch),
        Boundary::BackendHandler => {
            let backend_flags = BackendFlags::builder()
                .pagination(flags.pagination)
                .openapi(flags.openapi)
                .rate_limit(flags.rate_limit)
                .problem_details(flags.problem_details)
                .idempotency_key(flags.idempotency_key)
                .build();
            check_backend_boundary(content, &mut v, backend_flags);
        }
        Boundary::GatewayWorker => check_gateway_boundary(content, &mut v, flags.aip_4222),
        Boundary::WebhookHandler => {
            check_webhook_boundary(
                content,
                &mut v,
                flags.signature_verify,
                flags.timestamp_window,
            );
        }
        Boundary::DatabaseLayer => {
            check_database_boundary(content, &mut v, flags.tenant_filter, flags.rls_context);
        }
        Boundary::Unknown => {}
    }

    check_cross_boundary(content, &mut v, flags.jwt_verify);
    v
}

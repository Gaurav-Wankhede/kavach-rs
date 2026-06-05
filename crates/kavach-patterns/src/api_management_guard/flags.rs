//! Feature flag detection across API boundaries.

#[expect(
    clippy::struct_excessive_bools,
    reason = "flag aggregation struct; each bool is an independent detector signal"
)]
pub(super) struct Flags {
    pub authfetch: bool,
    pub pagination: bool,
    pub openapi: bool,
    pub signature_verify: bool,
    pub timestamp_window: bool,
    pub rate_limit: bool,
    pub aip_4222: bool,
    pub tenant_filter: bool,
    pub rls_context: bool,
    pub idempotency_key: bool,
    pub problem_details: bool,
    pub jwt_verify: bool,
}

impl Flags {
    pub(super) fn detect(content: &str) -> Self {
        Self {
            authfetch: content.contains("authFetch")
                || content.contains("apiClient.")
                || content.contains("api.client")
                || content.contains("useApi("),
            pagination: content.contains("limit:")
                || content.contains("cursor")
                || content.contains("page_size")
                || content.contains("PaginationParams")
                || content.contains("Page<"),
            openapi: content.contains("#[utoipa::path")
                || content.contains("@OpenApi")
                || content.contains("@openapi")
                || content.contains("ApiOperation"),
            signature_verify: content.contains("constructEvent")
                || content.contains("verify_signature")
                || content.contains("verify_webhook")
                || content.contains("hmac::verify")
                || content.contains("Stripe::Webhook::construct_event"),
            timestamp_window: content.contains("tolerance")
                || content.contains("timestamp_window")
                || content.contains("max_age"),
            rate_limit: content.contains("RateLimitLayer")
                || content.contains("governor::")
                || content.contains("tower_governor")
                || content.contains("@rateLimit")
                || content.contains("GovernorLayer"),
            aip_4222: content.contains("X-Request-Id")
                || content.contains("X-Request-Timestamp")
                || content.contains("X-Request-Platform"),
            tenant_filter: content.contains("tenant_id")
                || content.contains("organization_id")
                || content.contains("workspace_id"),
            rls_context: content.contains("SET LOCAL")
                || content.contains("set_config")
                || content.contains("app.tenant_id"),
            idempotency_key: content.contains("Idempotency-Key")
                || content.contains("x-idempotency-key")
                || content.contains("idempotency_key"),
            problem_details: content.contains("application/problem+json")
                || content.contains("ProblemDetails")
                || content.contains("RFC 9457"),
            jwt_verify: content.contains(".validate")
                || content.contains("Validation::default")
                || content.contains("verify_required_claims"),
        }
    }
}

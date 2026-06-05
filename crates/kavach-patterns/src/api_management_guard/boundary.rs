#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Boundary {
    FrontendComponent,
    BackendHandler,
    GatewayWorker,
    WebhookHandler,
    DatabaseLayer,
    Unknown,
}

pub(super) fn has_extension(path: &str, ext: &str) -> bool {
    path.to_lowercase().ends_with(&ext.to_lowercase())
}

pub(super) fn classify_boundary(path: &str, content: &str) -> Boundary {
    let p = path.to_lowercase();
    // Path-prefix checks FIRST so a .tsx in /api/handlers/ is BackendHandler not Frontend.
    if p.contains("/webhooks/")
        || p.contains("/webhook/")
        || has_extension(&p, "webhook.rs")
        || has_extension(&p, "webhooks.rs")
        || content.contains("Stripe-Signature")
        || content.contains("X-Hub-Signature")
    {
        return Boundary::WebhookHandler;
    }
    if p.contains("workers/")
        || p.contains("/cf-worker")
        || has_extension(&p, "wrangler.toml")
        || content.contains("ExecutionContext")
        || content.contains("addEventListener('fetch'")
    {
        return Boundary::GatewayWorker;
    }
    if p.contains("/handlers/")
        || p.contains("/routes/")
        || p.contains("/api/")
        || content.contains("axum::Router")
        || content.contains("#[tokio::main]")
        || content.contains("#[actix_web::")
        || content.contains("from fastapi")
    {
        return Boundary::BackendHandler;
    }
    if p.contains("/migrations/")
        || p.contains("/repository/")
        || p.contains("/repo/")
        || p.contains("/db/")
        || has_extension(&p, ".sql")
    {
        return Boundary::DatabaseLayer;
    }
    // Frontend extensions checked LAST after path-prefix routing.
    if has_extension(&p, ".tsx")
        || has_extension(&p, ".jsx")
        || has_extension(&p, ".vue")
        || has_extension(&p, ".svelte")
        || has_extension(&p, ".astro")
    {
        return Boundary::FrontendComponent;
    }
    Boundary::Unknown
}

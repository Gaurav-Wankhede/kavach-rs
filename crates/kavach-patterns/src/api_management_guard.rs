//! API Management & Boundary Contract Gate
//!
//! Enforces API contracts across 5 boundaries:
//!   A. Frontend → API   (must use authFetch wrapper, not bare `fetch()`)
//!   B. Backend handler   (versioned route, pagination, typed DTO, rate limit)
//!   C. Cloudflare/gateway (AIP-4222 headers, no CORS wildcard)
//!   D. Third-party webhook (signature verification + replay-window)
//!   E. Database row     (RLS context + `tenant_id` filter)
//!
//! SOURCES (verified 2026-05):
//! - <https://owasp.org/API-Security>/
//! - <https://developers.cloudflare.com/api-shield>/
//! - <https://docs.stripe.com/webhooks>
//! - <https://www.rfc-editor.org/rfc/rfc9457>
//! - <https://aip.dev/4222>

pub use self::detect::detect;
pub use self::types::{ApiSeverity, ApiViolation};

mod backend;
mod backend_flags;
mod boundary;
mod cross_boundary;
mod database;
mod detect;
mod flags;
mod frontend;
mod gateway;
mod patterns;
mod types;
mod webhook;

#[cfg(test)]
#[path = "api_management_guard_test.rs"]
mod tests;

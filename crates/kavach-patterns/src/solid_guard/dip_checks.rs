//! Dependency Inversion Principle (DIP) violation detection.

use crate::solid_guard::{SolidLetter, SolidSeverity, SolidViolation};

fn push(
    v: &mut Vec<SolidViolation>,
    severity: SolidSeverity,
    pattern: &'static str,
    fix: &'static str,
) {
    v.push(SolidViolation {
        severity,
        letter: SolidLetter::D,
        pattern,
        fix,
        line: 0,
    });
}

pub(super) fn check_concrete_client_param(
    p: &[regex::Regex],
    content: &str,
    v: &mut Vec<SolidViolation>,
) {
    if p.get(8).is_some_and(|re| re.is_match(content)) {
        push(
            v,
            SolidSeverity::P1Advisory,
            "dip-concrete-client-param",
            "pub fn parameter is a concrete sqlx/reqwest/mongodb client. Depend on a repository/gateway trait so callers can inject mocks.",
        );
    }
}

pub(super) fn check_concrete_service_field(
    p: &[regex::Regex],
    content: &str,
    v: &mut Vec<SolidViolation>,
) {
    if p.get(9).is_some_and(|re| re.is_match(content)) {
        push(
            v,
            SolidSeverity::P1Advisory,
            "dip-concrete-service-field",
            "Service struct holds concrete client. Use a generic <R: Repository> or Box<dyn Repository> field; high-level depends on abstraction.",
        );
    }
}

pub(super) fn check_axum_state_concrete(
    p: &[regex::Regex],
    content: &str,
    v: &mut Vec<SolidViolation>,
) {
    if p.get(13).is_some_and(|re| re.is_match(content)) {
        push(
            v,
            SolidSeverity::P1Advisory,
            "axum-dip-state-concrete",
            "Axum State<concrete-client> couples handler to driver. Use State<Arc<dyn Repository + Send + Sync>>; wire concrete impl in main via with_state.",
        );
    }
}

pub(super) fn check_axum_extension_concrete(
    p: &[regex::Regex],
    content: &str,
    v: &mut Vec<SolidViolation>,
) {
    if p.get(16).is_some_and(|re| re.is_match(content)) {
        push(
            v,
            SolidSeverity::P1Advisory,
            "axum-dip-extension-concrete",
            "Extension<concrete-client> bypasses compile-time State typing. Switch to State<Arc<dyn Repository>>; FromRef substates compose without runtime 500s.",
        );
    }
}

pub(super) fn check_domain_imports_infra(
    p: &[regex::Regex],
    file_path: &str,
    content: &str,
    v: &mut Vec<SolidViolation>,
) {
    if p.get(18).is_some_and(|re| re.is_match(content))
        && (file_path.contains("/domain/") || file_path.contains("/core/"))
    {
        push(
            v,
            SolidSeverity::P1Advisory,
            "dip-domain-imports-infra",
            "Domain layer imports crate::infra/persistence/adapters. Domain owns the trait; infra implements it.",
        );
    }
}

pub(super) fn check_lazy_global_client(
    p: &[regex::Regex],
    content: &str,
    v: &mut Vec<SolidViolation>,
) {
    if p.get(21).is_some_and(|re| re.is_match(content)) {
        push(
            v,
            SolidSeverity::P1Advisory,
            "dip-lazy-global-client",
            "lazy_static!/Lazy<concrete-client> = global concrete dependency. Inject via State<Arc<dyn Repository>>; globals can't be mocked or scoped per-test.",
        );
    }
}

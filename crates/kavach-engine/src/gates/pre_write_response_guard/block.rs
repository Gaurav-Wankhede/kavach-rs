//! P0 block: `#[serde(default)]` on a privilege field (`is_admin`, `role`,
//! `permission`, …) lets a missing field default to false/empty — privilege
//! escalation via omission. Fires only on backend handler/route/api `.rs` files.
use super::super::platform_guard_msg::build_block;
use super::super::platform_guard_paths::is_test;
use std::path::Path;

const PRIV_FIELDS: &[&str] = &["is_admin", "is_moderator", "is_owner", "role", "permission"];

/// True for non-test `.rs` files whose path marks them as a backend surface.
fn is_backend_rs(path: &str) -> bool {
    if !Path::new(path)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rs"))
    {
        return false;
    }
    path.contains("handler")
        || path.contains("route")
        || path.contains("controller")
        || path.contains("api")
}

/// `Some(reason)` when a privilege field carries `#[serde(default)]`.
pub(crate) fn check(file_path: &str, content: &str) -> Option<String> {
    if !is_backend_rs(file_path) || is_test(file_path) {
        return None;
    }
    let lc = content.to_lowercase();
    let has_serde_default = lc.contains("serde(default") || lc.contains("serde( default");
    if !has_serde_default {
        return None;
    }
    let mut p0: Vec<(&str, &str)> = Vec::new();
    for field in PRIV_FIELDS {
        if lc.contains(field) {
            p0.push(("PRIV_FIELD_SERDE_DEFAULT",
                "Remove #[serde(default)] from privilege fields (is_admin, role, permission) — \
                 a missing field defaults to false/empty, enabling privilege escalation via omission."));
            break;
        }
    }
    if p0.is_empty() {
        return None;
    }
    Some(build_block("RESPONSE_SECURITY_GUARD", &p0))
}

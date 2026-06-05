//! Auth-relevance detection by file-path and content patterns.

const AUTH_PATH_PATTERNS: &[&str] = &[
    "/auth/",
    "/grant/",
    "/token/",
    "/session/",
    "/login/",
    "/oauth/",
    "/gnap/",
    "handler",
    "middleware",
];

const AUTH_CONTENT_PATTERNS: &[&str] = &[
    "Authorization",
    "access_token",
    "Bearer",
    "client_id",
    "httpsig",
    "Signature-Input",
    "grant_request",
    "introspect",
];

/// True when the path or content matches a known auth pattern.
pub(super) fn is_auth_related(file_path: &str, content: &str) -> bool {
    let path_lower = file_path.to_lowercase();
    if AUTH_PATH_PATTERNS.iter().any(|p| path_lower.contains(p)) {
        return true;
    }
    let content_lower = content.to_lowercase();
    AUTH_CONTENT_PATTERNS
        .iter()
        .any(|p| content_lower.contains(&p.to_lowercase()))
}

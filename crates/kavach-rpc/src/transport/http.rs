// split: intentional - HTTP transport with random port + lockfile
use crate::lockfile;
use crate::state::AppState;
use http::{Request, Response, StatusCode};
use jsonrpsee::RpcModule;
use jsonrpsee::server::Server;
use std::net::SocketAddr;
use subtle::ConstantTimeEq;
use tower::ServiceBuilder;
use tower_http::validate_request::{ValidateRequest, ValidateRequestHeaderLayer};

/// Per-request bearer validator. Rejects any request whose `Authorization`
/// header is not exactly `Bearer <KAVACH_RPC_HTTP_TOKEN>`. Comparison is
/// constant-time (`subtle::ConstantTimeEq`) so a remote caller cannot recover
/// the token byte-by-byte via response-timing — the reason tower-http's own
/// `ValidateRequestHeaderLayer::bearer()` is deprecated (non-CT, "too basic").
#[derive(Clone)]
struct BearerAuth {
    token: String,
}

impl<B> ValidateRequest<B> for BearerAuth {
    type ResponseBody = jsonrpsee::server::HttpBody;

    fn validate(&mut self, request: &mut Request<B>) -> Result<(), Response<Self::ResponseBody>> {
        let presented = request
            .headers()
            .get(http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .unwrap_or("");
        // ct_eq over equal-length byte slices; length mismatch -> not equal,
        // still evaluated without early-out on the secret.
        let ok: bool = presented.as_bytes().ct_eq(self.token.as_bytes()).into();
        if ok {
            Ok(())
        } else {
            let mut resp = Response::new(jsonrpsee::server::HttpBody::empty());
            *resp.status_mut() = StatusCode::UNAUTHORIZED;
            resp.headers_mut().insert(
                http::header::WWW_AUTHENTICATE,
                http::HeaderValue::from_static("Bearer"),
            );
            Err(resp)
        }
    }
}

/// Starts the HTTP JSON-RPC transport, authenticating all requests via constant-time bearer token validation.
///
/// # Errors
/// Returns an error if `KAVACH_RPC_HTTP_TOKEN` is not set or empty (fail-closed), or if the server fails to bind
/// or is unable to write the lockfile.
pub async fn run(module: RpcModule<AppState>) -> std::io::Result<()> {
    // FIX: [auth_bypass CWE-306 / CSRF CWE-352] http.rs:8
    // SYMPTOM: the HTTP transport served EVERY RPC method over loopback TCP
    //          with zero authentication; any local process (or a browser via
    //          CSRF) could POST db.delete etc. The in-tree client only ever
    //          uses the Unix socket (client.rs:87) — HTTP has no legitimate
    //          consumer, so an unauthenticated RCE surface is pure exposure.
    // WHY5: an RPC surface conveys no authentication from its transport
    //       address; loopback is NOT authentication (CryptoNote takeover
    //       class). Every transport must enforce caller identity, matching
    //       the Unix path's 0o600 + SO_PEERCRED guarantee.
    // ROOT_CAUSE: HTTP transport had no per-startup secret / no auth.
    // RESEARCH: cwe.mitre.org/data/definitions/306.html; CryptoNote
    //           unauthenticated-localhost-RPC takeover.
    // SOLUTION: fail closed — refuse to start the HTTP transport unless the
    //           operator explicitly provisions KAVACH_RPC_HTTP_TOKEN. This
    //           removes the default unauthenticated surface entirely; the
    //           Unix socket remains the auth'd default transport.
    // Fail-closed on every non-Ok path: NotPresent (env unset), NotUnicode
    // (set but non-UTF-8 bytes), and empty/whitespace-only token all reject.
    // We log NotUnicode separately because it indicates a misconfigured shell
    // (raw bytes in env) and the operator deserves to see the cause.
    let token = match std::env::var("KAVACH_RPC_HTTP_TOKEN") {
        Ok(v) => v,
        Err(std::env::VarError::NotPresent) => String::new(),
        Err(std::env::VarError::NotUnicode(os)) => {
            tracing::warn!(
                target: "kavach_rpc::http",
                bytes = ?os,
                "KAVACH_RPC_HTTP_TOKEN contains non-UTF-8 bytes; treating as unset (fail-closed)"
            );
            String::new()
        }
    };
    if token.trim().is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "HTTP transport refused: set KAVACH_RPC_HTTP_TOKEN to a strong \
             secret to enable it. The Unix socket transport is the \
             authenticated default and requires no token.",
        ));
    }

    let bind_addr: SocketAddr = "127.0.0.1:0".parse().map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("parse addr: {e}"))
    })?;

    // Per-request bearer enforcement (defense-in-depth on top of the
    // startup-token fail-closed above). HTTP `Authorization` is an
    // HTTP-layer concern, so it is enforced via `set_http_middleware`
    // (a tower layer) — rejected before any JSON-RPC parsing, returning a
    // proper 401. (jsonrpsee 0.24 separates HTTP vs RPC middleware.)
    let auth_layer = ServiceBuilder::new().layer(ValidateRequestHeaderLayer::custom(BearerAuth {
        token: token.clone(),
    }));

    let server = Server::builder()
        .set_http_middleware(auth_layer)
        .build(bind_addr)
        .await
        .map_err(|e| std::io::Error::other(format!("build server: {e}")))?;

    let local = server
        .local_addr()
        .map_err(|e| std::io::Error::other(format!("local_addr: {e}")))?;
    let port = local.port();

    let lock_path = lockfile::write_lockfile(port, "http")
        .map_err(|e| std::io::Error::other(format!("lockfile: {e}")))?;
    tracing::info!(
        "kavach-rpc listening on http://{local} (lockfile: {})",
        lock_path.display()
    );

    let handle = server.start(module);

    let shutdown = tokio::signal::ctrl_c();
    tokio::select! {
        () = handle.clone().stopped() => {
            tracing::info!("server stopped");
        }
        _ = shutdown => {
            tracing::info!("ctrl-c received, shutting down");
            // handle.stop() returns AlreadyStoppedError when the server is
            // already winding down (e.g. dual SIGINT) — log + continue rather
            // than silently discard, so shutdown debugging stays honest.
            if let Err(e) = handle.stop() {
                tracing::warn!(error = %e, "handle.stop() failed (already stopped?)");
            }
        }
    }

    lockfile::remove_lockfile();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validator() -> BearerAuth {
        BearerAuth {
            token: "s3cr3t-token".to_owned(),
        }
    }

    fn req_with(auth: Option<&str>) -> Request<()> {
        let mut b = Request::builder().uri("/");
        if let Some(a) = auth {
            b = b.header(http::header::AUTHORIZATION, a);
        }
        match b.body(()) {
            Ok(r) => r,
            Err(e) => panic!("static test request must build: {e}"),
        }
    }

    #[test]
    fn rejects_absent_authorization_header() {
        let mut v = validator();
        let mut r = req_with(None);
        let res = v.validate(&mut r);
        let Err(resp) = res else {
            panic!("missing header must be rejected");
        };
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            resp.headers()
                .get(http::header::WWW_AUTHENTICATE)
                .map(|h| h.to_str().ok()),
            Some(Some("Bearer"))
        );
    }

    #[test]
    fn rejects_wrong_token() {
        let mut v = validator();
        let mut r = req_with(Some("Bearer not-the-token"));
        assert!(
            v.validate(&mut r).is_err(),
            "wrong token must be rejected (constant-time path)"
        );
    }

    #[test]
    fn rejects_non_bearer_scheme() {
        let mut v = validator();
        let mut r = req_with(Some("Basic s3cr3t-token"));
        assert!(
            v.validate(&mut r).is_err(),
            "only the Bearer scheme is accepted"
        );
    }

    #[test]
    fn accepts_correct_bearer_token() {
        let mut v = validator();
        let mut r = req_with(Some("Bearer s3cr3t-token"));
        assert!(v.validate(&mut r).is_ok(), "exact Bearer token must pass");
    }

    #[test]
    fn rejects_token_prefix_collision() {
        // A token that is a prefix of the real one must fail (ct_eq is
        // length-sensitive — guards against truncation-style probing).
        let mut v = validator();
        let mut r = req_with(Some("Bearer s3cr3t"));
        assert!(v.validate(&mut r).is_err());
    }
}

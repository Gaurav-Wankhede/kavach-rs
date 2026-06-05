// split: intentional - Unix-domain-socket JSON-RPC transport
// Per research.engine-async-blocker (id=3759): kavach-engine gates run sync inside
// Claude Code hooks. Spawning a tokio runtime per gate adds 50-200ms latency.
// Unix socket round-trip to a long-running kavach-rpc daemon adds <5ms.
// One persistent kavach-rpc process holds the SurrealDB connection; gates send
// JSON-RPC requests over UDS and parse line-delimited responses synchronously.
use crate::lockfile;
use crate::state::AppState;
use jsonrpsee::RpcModule;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
// SOURCE: https://docs.rs/tokio-util/0.7/tokio_util/sync/struct.CancellationToken.html
// CancellationToken cascades cleanly to spawned per-client tasks via child_token().
use tokio_util::sync::CancellationToken;

#[must_use]
pub fn default_socket_path() -> PathBuf {
    if cfg!(target_os = "macos") {
        dirs::home_dir().map_or_else(
            || PathBuf::from("/tmp/kavach-rpc.sock"),
            |h| h.join("Library/Application Support/SharedAI/kavach-rpc.sock"),
        )
    } else if cfg!(target_os = "windows") {
        // Windows AF_UNIX path mirrors HTTP fallback location.
        dirs::data_local_dir().map_or_else(
            || PathBuf::from("C:\\Users\\Public\\SharedAI\\kavach-rpc.sock"),
            |d| d.join("SharedAI\\kavach-rpc.sock"),
        )
    } else {
        dirs::data_dir().map_or_else(
            || PathBuf::from("/tmp/kavach-rpc.sock"),
            |d| d.join("shared-ai/kavach-rpc.sock"),
        )
    }
}

/// Runs the Unix-domain socket JSON-RPC server.
///
/// # Errors
/// Returns [`std::io::Error`] on socket bind failure, permission setup, lockfile I/O,
/// or other filesystem operations.
#[expect(
    clippy::too_many_lines,
    reason = "single linear dispatcher: socket setup + accept loop + shutdown cleanup"
)]
pub async fn run(module: RpcModule<AppState>) -> std::io::Result<()> {
    let path = default_socket_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
        // FIX: [auth_bypass/TOCTOU CWE-367] unix.rs:44
        // SYMPTOM: socket world-accessible (per umask) in the window between
        //          bind() and set_permissions(0o600); also rename-substitution.
        // WHY5: confidentiality of a path socket must be enforced at CREATION
        //       (umask) and by the CONTAINING DIRECTORY — never by a later chmod.
        // ROOT_CAUSE: protection applied after a kernel-level creation race.
        // RESEARCH: github.com/tokio-rs/tokio#4422 (documents this exact race);
        //           man7.org/.../unix.7 ("pathname sockets honor the permissions
        //           of the directory they are in"). The 0o700 parent dir is the
        //           real boundary: the socket is unreachable during any window.
        // SOLUTION: lock the parent dir owner-only BEFORE bind.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
        }
    }
    // Race-free stale-socket cleanup: skip the exists() probe (CWE-367 TOCTOU)
    // and unconditionally remove, tolerating NotFound.
    match std::fs::remove_file(&path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            tracing::warn!(
                target: "kavach_rpc::transport::unix",
                path = %path.display(),
                error = %e,
                "stale socket cleanup failed; bind() will surface the real error"
            );
        }
    }

    // Atomic restricted socket creation: the kernel applies the umask at bind()
    // time, so the socket is never momentarily world-accessible. rustix::umask
    // is a safe wrapper (no FFI/unsafe — upholds forbid(unsafe_code)). Startup
    // is single-threaded here so the brief process-global umask change is sound.
    #[cfg(unix)]
    let prev_umask = rustix::process::umask(rustix::fs::Mode::from_raw_mode(0o177));
    let listener = UnixListener::bind(&path);
    #[cfg(unix)]
    rustix::process::umask(prev_umask);
    let listener = listener?;

    // Defense-in-depth: explicit owner-only socket mode (0o600) on top of the
    // umask-at-creation guarantee and the 0o700 parent directory boundary.
    // RESEARCH: CVE-2026-39813 (FortiSandbox JRPC, CVSS 9.1); CryptoNote
    //           unauthenticated-RPC takeover; man7.org/.../unix.7 SO_PEERCRED.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&path, perms)?;
    }

    let module = Arc::new(module);

    if let Err(e) = lockfile::write_lockfile(0, "unix") {
        tracing::warn!("lockfile write failed (non-fatal): {e}");
    }
    tracing::info!("kavach-rpc listening on unix socket: {}", path.display());

    // Root cancellation token; ctrl-c trips it and every child token follows.
    let cancel = CancellationToken::new();
    let signal_cancel = cancel.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            tracing::info!("ctrl-c received, signalling shutdown via CancellationToken");
            signal_cancel.cancel();
        }
    });

    loop {
        tokio::select! {
            accept = listener.accept() => {
                match accept {
                    Ok((stream, _addr)) => {
                        // Defense-in-depth: even with 0o600, verify the peer's uid
                        // matches this daemon's effective uid via SO_PEERCRED.
                        // Drops cross-uid connections (e.g. shared-runner edge cases)
                        // before any RPC method dispatch. Same class as CVE-2026-39813.
                        #[cfg(unix)]
                        {
                            match stream.peer_cred() {
                                Ok(cred) => {
                                    // rustix::process::geteuid is safe (no unsafe
                                    // block) and already in the workspace tree.
                                    let euid = rustix::process::geteuid().as_raw();
                                    if cred.uid() != euid {
                                        tracing::warn!(
                                            peer_uid = cred.uid(),
                                            daemon_euid = euid,
                                            "rejecting RPC connection: peer uid mismatch"
                                        );
                                        continue;
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!("rejecting RPC connection: peer_cred failed: {e}");
                                    continue;
                                }
                            }
                        }
                        let module = Arc::clone(&module);
                        let child_cancel = cancel.child_token();
                        tokio::spawn(async move {
                            tokio::select! {
                                res = handle_client(stream, module) => {
                                    if let Err(e) = res {
                                        tracing::warn!("unix client error: {e}");
                                    }
                                }
                                () = child_cancel.cancelled() => {
                                    tracing::debug!("unix client cancelled mid-flight");
                                }
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("accept error: {e}");
                    }
                }
            }
            () = cancel.cancelled() => {
                tracing::info!("shutdown signalled, dropping accept loop");
                break;
            }
        }
    }

    // Shutdown cleanup: socket removal is best-effort. NotFound is benign;
    // other errors are logged but never propagated — shutdown must succeed.
    match std::fs::remove_file(&path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => tracing::warn!(
            target: "kavach_rpc::transport::unix",
            path = %path.display(),
            error = %e,
            "socket removal during shutdown failed"
        ),
    }
    lockfile::remove_lockfile();
    Ok(())
}

async fn handle_client(
    stream: UnixStream,
    module: Arc<RpcModule<AppState>>,
) -> std::io::Result<()> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match module.raw_json_request(trimmed, 1).await {
            Ok((response, _stream)) => {
                write_half.write_all(response.as_bytes()).await?;
                write_half.write_all(b"\n").await?;
                write_half.flush().await?;
            }
            Err(e) => {
                tracing::error!("rpc dispatch error: {e}");
            }
        }
    }
    Ok(())
}

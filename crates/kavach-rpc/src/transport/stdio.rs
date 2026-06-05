// split: intentional - line-delimited JSON-RPC stdio loop
// CRITICAL: stdout is the protocol channel; all logs go to stderr.
use crate::state::AppState;
use jsonrpsee::RpcModule;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Run the stdio JSON-RPC transport loop.
///
/// Reads line-delimited JSON-RPC requests from stdin and writes responses to stdout.
///
/// # Errors
///
/// Returns an error if stdin/stdout operations fail.
pub async fn run(module: RpcModule<AppState>) -> std::io::Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut out = stdout;
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            tracing::info!("stdin EOF, shutting down");
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match module.raw_json_request(trimmed, 1).await {
            Ok((response, _stream)) => {
                out.write_all(response.as_bytes()).await?;
                out.write_all(b"\n").await?;
                out.flush().await?;
            }
            Err(e) => {
                tracing::error!("rpc dispatch error: {e}");
            }
        }
    }

    Ok(())
}

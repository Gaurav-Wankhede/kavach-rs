//! `kavach db embed-backfill` — populate `memories.embedding` for NULL rows (Phase 5).

use kavach_db::turso::TursoStore;
use kavach_db::turso::vector::VectorStore as _;

/// Run the backfill. Returns process exit code.
pub(crate) fn run(batch: usize, json: bool) -> i32 {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: tokio runtime: {e}");
            return 1;
        }
    };
    rt.block_on(async_run(batch, json))
}

async fn async_run(batch: usize, json: bool) -> i32 {
    let store = match TursoStore::open_default().await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: open turso store: {e}");
            return 1;
        }
    };
    match store.backfill_embeddings(batch).await {
        Ok(n) => {
            if json {
                println!("{{\"backfilled\":{n},\"batch\":{batch}}}");
            } else {
                println!("backfilled {n} embedding(s) (batch {batch})");
            }
            0
        }
        Err(e) => {
            eprintln!("error: backfill: {e}");
            1
        }
    }
}

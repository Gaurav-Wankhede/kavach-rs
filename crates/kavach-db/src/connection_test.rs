use super::*;

#[tokio::test]
async fn test_open_memory() -> Result<()> {
    let db = open_memory().await?;
    // The schema bootstrap must have created the core tables.
    let mut rows = db
        .exec_params(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'memories'",
            (),
        )
        .await?;
    let found = rows.next().await?.is_some();
    assert!(found, "open_memory must bootstrap the memories table");
    Ok(())
}

use super::*;

#[tokio::test]
async fn test_open_memory() -> Result<()> {
    let db = open_memory().await?;
    let info: Option<serde_json::Value> = db.query("INFO FOR DB").await?.take(0)?;
    info.ok_or_else(|| {
        crate::error::Error::RecordNotFound("INFO FOR DB returned empty result".to_owned())
    })?;
    Ok(())
}

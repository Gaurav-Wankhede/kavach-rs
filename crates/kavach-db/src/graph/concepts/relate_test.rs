use super::relate_concepts;

#[tokio::test]
async fn test_relate_concepts_idempotent() {
    let store = crate::open_memory().await.expect("in-memory db setup");

    // First relation
    let res1 = relate_concepts(&store, "Authentication", "is_a", "Security").await;
    assert!(res1.is_ok());

    // Duplicate relation under retry/replay must succeed idempotently
    let res2 = relate_concepts(&store, "Authentication", "is_a", "Security").await;
    assert!(res2.is_ok());

    // Empty concept names must fail-closed
    let res_empty = relate_concepts(&store, "", "is_a", "Security").await;
    assert!(res_empty.is_err());
}

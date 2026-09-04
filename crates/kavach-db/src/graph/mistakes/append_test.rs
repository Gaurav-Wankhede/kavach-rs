// Idempotency proofs: a same-turn re-file converges to one event; a distinct
// turn stays distinct. Pins the RCA fix for the MISTAKE_RECORD_FAILED re-file
// double-count (see append.rs header).
use super::{append_loophole_event, append_mistake_event, event_key};
use crate::open_memory;

#[test]
fn event_key_is_deterministic_and_turn_sensitive() {
    let a = event_key("mev", &["gate", "fix", "banned"], "sess", 0);
    let b = event_key("mev", &["gate", "fix", "banned"], "sess", 0);
    let c = event_key("mev", &["gate", "fix", "banned"], "sess", 1);
    assert_eq!(a, b, "identical identity+session+turn must key the same");
    assert_ne!(a, c, "a distinct turn must key differently");
}

#[tokio::test]
async fn same_turn_refile_converges_to_one_event() {
    let db = open_memory().await.expect("open in-memory db");
    let first = append_mistake_event(&db, "gate", "fix", "banned", "sess", None, 0)
        .await
        .expect("first file");
    let second = append_mistake_event(&db, "gate", "fix", "banned", "sess", None, 0)
        .await
        .expect("re-file");
    assert_eq!(first, second, "identical re-file must converge to the same event id");
}

#[tokio::test]
async fn distinct_turn_stays_distinct() {
    let db = open_memory().await.expect("open in-memory db");
    let t0 = append_mistake_event(&db, "gate", "fix", "banned", "sess", None, 0)
        .await
        .expect("turn 0");
    let t1 = append_mistake_event(&db, "gate", "fix", "banned", "sess", None, 1)
        .await
        .expect("turn 1");
    assert_ne!(t0, t1, "a distinct turn must yield a distinct event id");
}

#[tokio::test]
async fn loophole_event_converges_the_same_way() {
    let db = open_memory().await.expect("open in-memory db");
    let first = append_loophole_event(&db, "injection", "site", "sess", None, 0)
        .await
        .expect("first file");
    let second = append_loophole_event(&db, "injection", "site", "sess", None, 0)
        .await
        .expect("re-file");
    assert_eq!(first, second, "loophole re-file must converge too");
}

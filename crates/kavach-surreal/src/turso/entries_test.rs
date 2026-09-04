//! EntryStore tests — Phase 1 acceptance (PRD §4 Phase 1) + proptests.

use super::entries::EntryStore;
use super::TursoStore;
use crate::dual_write::MemoryEntry;
use proptest::prop_assert;
use proptest::prop_assert_eq;
use surrealdb_types::{RecordId, RecordIdKey};

fn entry(project: &str, category: &str, key: &str, title: &str) -> MemoryEntry {
    MemoryEntry {
        id: None,
        project: RecordId::new("project", RecordIdKey::String(project.to_owned())),
        category: Some(category.to_owned()),
        entry_key: key.to_owned(),
        title: title.to_owned(),
        content: format!("body of {title}"),
        status: Some("todo".into()),
        entry_status: Some("todo".into()),
        tags: Some(vec!["rust".into(), "test".into()]),
        decay_score: None,
        access_count: Some(0),
        priority: None,
        lane: None,
        exec_prompt: None,
        occupied_by: None,
        occupied_until: None,
        created_at: None,
        updated_at: None,
    }
}

#[tokio::test]
async fn upsert_get_roundtrip_preserves_fields() {
    let dir = tempfile::TempDir::new().unwrap();
    let s = TursoStore::open(dir.path().join("rt.db")).await.unwrap();
    let e = entry("demo", "decision", "use-turso", "Adopt embedded storage");
    s.upsert_entry(&e).await.unwrap();
    let got = s
        .get_by_key("demo", "decision", "use-turso")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(got.title, "Adopt embedded storage");
    assert_eq!(got.entry_key, "use-turso");
    assert_eq!(
        got.tags.as_deref(),
        Some(&["rust".to_owned(), "test".to_owned()][..])
    );
    assert_eq!(got.status.as_deref(), Some("todo"));
}

#[tokio::test]
async fn upsert_twice_updates_not_duplicates() {
    let dir = tempfile::TempDir::new().unwrap();
    let s = TursoStore::open(dir.path().join("dedup.db")).await.unwrap();
    s.upsert_entry(&entry("p", "research", "k1", "v1")).await.unwrap();
    let mut e2 = entry("p", "research", "k1", "v2-renamed");
    e2.content = "updated body".into();
    s.upsert_entry(&e2).await.unwrap();
    let got = s.get_by_key("p", "research", "k1").await.unwrap().unwrap();
    assert_eq!(got.title, "v2-renamed");
    assert_eq!(s.list_by_project("p").await.unwrap().len(), 1);
}

#[tokio::test]
async fn status_cas_transitions_only_from_expected() {
    let dir = tempfile::TempDir::new().unwrap();
    let s = TursoStore::open(dir.path().join("cas.db")).await.unwrap();
    s.upsert_entry(&entry("p", "roadmap", "card", "A card")).await.unwrap();
    assert!(s
        .update_status_cas("p", "roadmap", "card", "todo", "in_progress")
        .await
        .unwrap());
    // stale CAS from `todo` must fail now that status is `in_progress`
    assert!(!s
        .update_status_cas("p", "roadmap", "card", "todo", "done")
        .await
        .unwrap());
    let got = s.get_by_key("p", "roadmap", "card").await.unwrap().unwrap();
    assert_eq!(got.status.as_deref(), Some("in_progress"));
}

#[tokio::test]
async fn delete_removes_row_and_fts() {
    let dir = tempfile::TempDir::new().unwrap();
    let s = TursoStore::open(dir.path().join("del.db")).await.unwrap();
    s.upsert_entry(&entry("p", "pattern", "pat1", "Unique pattern name"))
        .await
        .unwrap();
    assert_eq!(s.delete_by_key("p", "pattern", "pat1").await.unwrap(), 1);
    assert!(s.get_by_key("p", "pattern", "pat1").await.unwrap().is_none());
    // FTS must not return ghosts
    let hits = s
        .list_with_filter("p", Some("pattern"), None, Some("Unique pattern"), 10)
        .await
        .unwrap();
    assert!(hits.is_empty());
}

proptest::proptest! {
    // Property: N distinct keys upserted -> exactly N rows; every key retrievable.
    #[test]
    fn cardinality_invariant(keys in proptest::collection::hash_set("[a-z0-9_-]{1,12}", 1..25)) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let dir = tempfile::TempDir::new().unwrap();
            let s = TursoStore::open(dir.path().join("prop.db")).await.unwrap();
            let n = keys.len();
            for k in &keys {
                s.upsert_entry(&entry("pp", "app_spec", k, k)).await.unwrap();
            }
            let all = s.list_by_project("pp").await.unwrap();
            prop_assert_eq!(all.len(), n);
            for k in &keys {
                prop_assert!(s.get_by_key("pp", "app_spec", k).await.unwrap().is_some());
            }
            Ok(())
        })?;
    }

    // Property: update never loses the newest title (last-write-wins on composite key).
    #[test]
    fn last_write_wins(key in "[a-z]{4}", titles in proptest::collection::vec("[A-Za-z ]{5,30}", 1..6)) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let dir = tempfile::TempDir::new().unwrap();
            let s = TursoStore::open(dir.path().join("lww.db")).await.unwrap();
            let last = titles.last().unwrap().clone();
            for t in &titles {
                s.upsert_entry(&entry("q", "decision", &key, t)).await.unwrap();
            }
            let got = s.get_by_key("q", "decision", &key).await.unwrap().unwrap();
            prop_assert_eq!(&got.title, &last);
            Ok(())
        })?;
    }
}

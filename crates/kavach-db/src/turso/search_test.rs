//! SearchStore tests — Phase 2 acceptance (PRD §4 Phase 2).

use super::TursoStore;
use super::entries::EntryStore;
use super::search::{SearchStore, fts5_query};
use crate::dual_write::MemoryEntry;
use proptest::prop_assert_eq;

fn entry(project: &str, category: &str, key: &str, title: &str, content: &str) -> MemoryEntry {
    MemoryEntry {
        id: None,
        project: project.to_owned(),
        category: Some(category.to_owned()),
        entry_key: key.to_owned(),
        title: title.to_owned(),
        content: content.to_owned(),
        status: Some("todo".into()),
        entry_status: Some("todo".into()),
        tags: None,
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
async fn fts_finds_title_and_content_matches() {
    let dir = tempfile::TempDir::new().unwrap();
    let s = TursoStore::open(dir.path().join("fts.db")).await.unwrap();
    s.upsert_entry(&entry(
        "p",
        "decision",
        "decision.use-turso",
        "Adopt embedded Turso",
        "libSQL storage replaces the TursoStoreDB server hop",
    ))
    .await
    .unwrap();
    s.upsert_entry(&entry(
        "p",
        "research",
        "research.wal-size",
        "WAL checkpoint hygiene",
        "keep the WAL small; checkpoint at 512 pages",
    ))
    .await
    .unwrap();

    // Title hit.
    let title_hits = s
        .fts_rank_category("p", "decision", "turso", 10)
        .await
        .unwrap();
    assert_eq!(title_hits, vec!["decision.use-turso".to_owned()]);

    // Content hit in another category.
    let content_hits = s
        .fts_rank_category("p", "research", "checkpoint", 10)
        .await
        .unwrap();
    assert_eq!(content_hits, vec!["research.wal-size".to_owned()]);

    // Category isolation: decision term must not surface under research.
    let isolated = s
        .fts_rank_category("p", "research", "turso", 10)
        .await
        .unwrap();
    assert!(isolated.is_empty());
}

#[tokio::test]
async fn fts_orders_by_relevance_not_insertion() {
    let dir = tempfile::TempDir::new().unwrap();
    let s = TursoStore::open(dir.path().join("fts2.db")).await.unwrap();
    // Inserted FIRST but mentions the term once; the second entry is denser.
    s.upsert_entry(&entry(
        "p",
        "pattern",
        "pattern.a",
        "Note",
        "mentions rust once",
    ))
    .await
    .unwrap();
    s.upsert_entry(&entry(
        "p",
        "pattern",
        "pattern.b",
        "Rust ownership deep dive",
        "rust rust rust borrowing and lifetimes in rust",
    ))
    .await
    .unwrap();
    let hits = s
        .fts_rank_category("p", "pattern", "rust", 10)
        .await
        .unwrap();
    assert_eq!(hits.first().map(String::as_str), Some("pattern.b"));
}

#[tokio::test]
async fn search_corpus_fuses_across_categories() {
    let dir = tempfile::TempDir::new().unwrap();
    let s = TursoStore::open(dir.path().join("fts3.db")).await.unwrap();
    s.upsert_entry(&entry(
        "p",
        "decision",
        "decision.embed",
        "Embed libSQL now",
        "kill the ws hop",
    ))
    .await
    .unwrap();
    s.upsert_entry(&entry(
        "p",
        "roadmap",
        "roadmap.embed-task",
        "Embed libSQL in gates",
        "swap the storage backend",
    ))
    .await
    .unwrap();
    s.upsert_entry(&entry(
        "p",
        "pattern",
        "pattern.unrelated",
        "Kanban lanes",
        "dispatch order",
    ))
    .await
    .unwrap();

    let fused = s.search_corpus("p", "embed libsql", 5).await.unwrap();
    assert!(!fused.is_empty());
    // Both matching entries surface; unrelated does not.
    let ids: Vec<&str> = fused.iter().map(|h| h.id.as_str()).collect();
    assert!(ids.contains(&"decision.embed"));
    assert!(ids.contains(&"roadmap.embed-task"));
    assert!(!ids.contains(&"pattern.unrelated"));
    // RRF score ordering: descending.
    let scores: Vec<f64> = fused.iter().map(|h| h.score).collect();
    let mut sorted = scores.clone();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());
    assert_eq!(scores, sorted);
}

#[tokio::test]
async fn empty_query_yields_no_hits() {
    let dir = tempfile::TempDir::new().unwrap();
    let s = TursoStore::open(dir.path().join("fts4.db")).await.unwrap();
    let fused = s.search_corpus("p", "   ", 5).await.unwrap();
    assert!(fused.is_empty());
}

#[tokio::test]
async fn search_corpus_project_prioritizes_scoped_project_and_handles_hyphens() {
    let dir = tempfile::TempDir::new().unwrap();
    let s = TursoStore::open(dir.path().join("fts_project.db")).await.unwrap();
    s.upsert_entry(&entry(
        "webfind",
        "decision",
        "decision.webfind-system-one",
        "Jev-Style System 1 Extractive Classifier Architecture",
        "two-stage extraction pipeline with confidence gates",
    ))
    .await
    .unwrap();
    s.upsert_entry(&entry(
        "other-project",
        "decision",
        "decision.other-system-one",
        "Other System 1 Architecture",
        "generic system architecture",
    ))
    .await
    .unwrap();

    // Query with hyphens, numbers, and multiple terms.
    let hits = crate::search_corpus_project(&s, Some("webfind"), "Jev-Style System 1", 5)
        .await
        .unwrap();
    assert!(!hits.is_empty());
    assert_eq!(hits[0].id, "decision.webfind-system-one");

    // Scoped project priorities: "System" matches both, but webfind must rank first.
    let hits_system = crate::search_corpus_project(&s, Some("webfind"), "System", 5)
        .await
        .unwrap();
    assert!(!hits_system.is_empty());
    assert_eq!(hits_system[0].id, "decision.webfind-system-one");
}

proptest::proptest! {
    // Property: the escaper never panics and always yields a balanced-quotes
    // MATCH fragment (even number of double quotes).
    #[test]
    fn escaper_is_injection_safe(q in "[a-zA-Z0-9\"'()*:\\- ]{0,40}") {
        let escaped = fts5_query(&q);
        let quotes = escaped.matches('"').count();
        prop_assert_eq!(quotes % 2, 0);
    }
}

#[test]
fn lexer_fts_output_matches_legacy_term_stream_exactly() {
    // Parity vectors for the lexer rewrite: identical MATCH strings to the old
    // `split_whitespace` + per-word quoting (verified by hand against it).
    for (raw, want) in [
        ("", ""),
        ("   ", ""),
        ("foo", "\"foo\""),
        ("foo bar", "\"foo\" \"bar\""),
        ("  foo   bar  ", "\"foo\" \"bar\""),
        ("foo \"bar\" baz", "\"foo\" \"\"\"bar\"\"\" \"baz\""),
        ("say \"hi\"", "\"say\" \"\"\"hi\"\"\""),
        ("todo done", "\"todo\" \"done\""),
        ("héllo wörk", "\"héllo\" \"wörk\""),
    ] {
        assert_eq!(fts5_query(raw), want, "raw={raw:?}");
    }
}

//! Unit tests for migration normalizer, batch import, and validation.
use super::types::{EdgeRecord, EntityRecord, ImportOpts, MemoryRecord, NormalizedRecord, SourceCounts};
use super::{import_records, normalize_record, validate_store};
use crate::turso::TursoStore;
use crate::turso::entries::EntryStore;

#[test]
fn test_normalize_record_memory() {
    let raw = serde_json::json!({
        "table": "decision",
        "project": "project:myapp",
        "entry_key": "decision:auth-token",
        "title": "Use JWT tokens",
        "content": "Stateless auth with 15m expiration",
        "status": "done",
        "tags": ["auth", "security"],
        "access_count": 5
    });
    let norm = normalize_record(&raw).expect("normalize memory");
    match norm {
        NormalizedRecord::Memory(m) => {
            assert_eq!(m.project, "myapp");
            assert_eq!(m.category, "decision");
            assert_eq!(m.entry_key, "auth-token");
            assert_eq!(m.title, "Use JWT tokens");
            assert_eq!(m.tags, vec!["auth", "security"]);
            assert_eq!(m.access_count, 5);
        }
        _ => panic!("expected NormalizedRecord::Memory"),
    }
}

#[test]
fn test_normalize_record_edge() {
    let raw = serde_json::json!({
        "table": "depends_on",
        "in": "entity:task_b",
        "out": "entity:task_a",
        "weight": 1.5
    });
    let norm = normalize_record(&raw).expect("normalize edge");
    match norm {
        NormalizedRecord::Edge(e) => {
            assert_eq!(e.from_id, "entity:task_b");
            assert_eq!(e.to_id, "entity:task_a");
            assert_eq!(e.rel_type, "depends_on");
            assert!((e.weight - 1.5).abs() < f64::EPSILON);
        }
        _ => panic!("expected NormalizedRecord::Edge"),
    }
}

#[tokio::test]
async fn test_import_and_validate_roundtrip() {
    let dir = tempfile::TempDir::new().unwrap();
    let db_path = dir.path().join("mig_test.db");
    let store = TursoStore::open(&db_path).await.unwrap();
    let records = vec![
        NormalizedRecord::Memory(Box::new(MemoryRecord {
            project: "p1".to_owned(),
            category: "decision".to_owned(),
            entry_key: "k1".to_owned(),
            title: "T1".to_owned(),
            content: "Content 1".to_owned(),
            status: Some("done".to_owned()),
            entry_status: None,
            tags: vec!["tag1".to_owned()],
            decay_score: None,
            access_count: 0,
            priority: Some(1),
            lane: None,
            exec_prompt: None,
            occupied_by: None,
            occupied_until: None,
            created_at: Some("2026-09-01T00:00:00Z".to_owned()),
            updated_at: Some("2026-09-01T00:00:00Z".to_owned()),
        })),
        NormalizedRecord::Entity(EntityRecord {
            id: "e1".to_owned(),
            entity_type: "concept".to_owned(),
            name: "concept1".to_owned(),
            properties: None,
            content_hash: None,
            project: Some("p1".to_owned()),
        }),
        NormalizedRecord::Entity(EntityRecord {
            id: "e2".to_owned(),
            entity_type: "concept".to_owned(),
            name: "concept2".to_owned(),
            properties: None,
            content_hash: None,
            project: Some("p1".to_owned()),
        }),
        NormalizedRecord::Edge(EdgeRecord {
            from_id: "e1".to_owned(),
            rel_type: "references".to_owned(),
            to_id: "e2".to_owned(),
            weight: 1.0,
        }),
    ];
    let opts = ImportOpts { batch_size: 50, overwrite: true, dry_run: false };
    let rep = import_records(&store, &records, opts).await.unwrap();
    assert_eq!(rep.memory_count, 1);
    assert_eq!(rep.entity_count, 2);
    assert_eq!(rep.edge_count, 1);
    let expected = SourceCounts {
        memory: 1,
        entity: 2,
        edge: 1,
        project: 0,
        part: 0,
        event: 0,
    };
    let val = validate_store(&store, &expected).await.unwrap();
    assert!(val.is_clean);
    assert!(val.dangling_edges.is_empty());
    assert!(val.fts_matched);
}

#[tokio::test]
async fn test_dangling_edge_detected() {
    let dir = tempfile::TempDir::new().unwrap();
    let db_path = dir.path().join("dangling.db");
    let store = TursoStore::open(&db_path).await.unwrap();
    let records = vec![
        NormalizedRecord::Entity(EntityRecord {
            id: "e1".to_owned(),
            entity_type: "concept".to_owned(),
            name: "concept1".to_owned(),
            properties: None,
            content_hash: None,
            project: None,
        }),
        NormalizedRecord::Edge(EdgeRecord {
            from_id: "e1".to_owned(),
            rel_type: "references".to_owned(),
            to_id: "missing_e2".to_owned(),
            weight: 1.0,
        }),
    ];
    let opts = ImportOpts { batch_size: 50, overwrite: true, dry_run: false };
    let _ = import_records(&store, &records, opts).await.unwrap();
    let expected = SourceCounts {
        memory: 0,
        entity: 1,
        edge: 1,
        project: 0,
        part: 0,
        event: 0,
    };
    let val = validate_store(&store, &expected).await.unwrap();
    assert!(!val.is_clean);
    assert_eq!(val.dangling_edges.len(), 1);
}

#[tokio::test]
async fn test_quote_injection_safe() {
    let dir = tempfile::TempDir::new().unwrap();
    let db_path = dir.path().join("injection.db");
    let store = TursoStore::open(&db_path).await.unwrap();
    let records = vec![NormalizedRecord::Memory(Box::new(MemoryRecord {
        project: "inject_p".to_owned(),
        category: "decision".to_owned(),
        entry_key: "k1".to_owned(),
        title: "'; DROP TABLE memories; --".to_owned(),
        content: "Malicious input test".to_owned(),
        status: None,
        entry_status: None,
        tags: vec![],
        decay_score: None,
        access_count: 0,
        priority: None,
        lane: None,
        exec_prompt: None,
        occupied_by: None,
        occupied_until: None,
        created_at: None,
        updated_at: None,
    }))];
    let opts = ImportOpts { batch_size: 10, overwrite: true, dry_run: false };
    import_records(&store, &records, opts).await.unwrap();
    let mems = store.list_all().await.unwrap();
    assert_eq!(mems.len(), 1);
    assert_eq!(mems[0].title, "'; DROP TABLE memories; --");
}

#[test]
fn test_normalize_endpoint_formats() {
    use super::export::normalize_endpoint;
    assert_eq!(normalize_endpoint("entity:00102z3nqek0gu0fs8nj"), "entity:00102z3nqek0gu0fs8nj");
    assert_eq!(
        normalize_endpoint("`RecordId { table`: `Table(\"entity\"), key: String(\"amjap2ghdqap5pjmveh9\") }`"),
        "entity:amjap2ghdqap5pjmveh9"
    );
    assert_eq!(
        normalize_endpoint("RecordId { table: Table(\"entity\"), key: String(\"xyz\") }"),
        "entity:xyz"
    );
    assert_eq!(
        normalize_endpoint("entity:`skill:durable_objects.bindings`"),
        "entity:skill:durable_objects.bindings"
    );
    assert_eq!(
        normalize_endpoint("roadmap:`String(\"nicole-carpenter\"):roadmap.unit.backend.cursor`"),
        "roadmap:nicole-carpenter:roadmap.unit.backend.cursor"
    );
    assert_eq!(normalize_endpoint("citation:1cmrc2u6s2h3rnpfp8zx"), "citation:1cmrc2u6s2h3rnpfp8zx");
}

#[tokio::test]
async fn test_validate_store_with_memory_and_project_endpoints() {
    let dir = tempfile::TempDir::new().unwrap();
    let db_path = dir.path().join("mem_edge.db");
    let store = TursoStore::open(&db_path).await.unwrap();
    let records = vec![
        NormalizedRecord::Memory(Box::new(MemoryRecord {
            project: "proj1".to_owned(),
            category: "roadmap".to_owned(),
            entry_key: "unit.task1".to_owned(),
            title: "Task 1".to_owned(),
            content: "Content".to_owned(),
            status: Some("done".to_owned()),
            entry_status: None,
            tags: vec![],
            decay_score: None,
            access_count: 0,
            priority: None,
            lane: None,
            exec_prompt: None,
            occupied_by: None,
            occupied_until: None,
            created_at: None,
            updated_at: None,
        })),
        NormalizedRecord::Entity(EntityRecord {
            id: "entity:c1".to_owned(),
            entity_type: "concept".to_owned(),
            name: "concept1".to_owned(),
            properties: None,
            content_hash: None,
            project: None,
        }),
        NormalizedRecord::Edge(EdgeRecord {
            from_id: "roadmap:proj1:unit.task1".to_owned(),
            rel_type: "implements".to_owned(),
            to_id: "entity:c1".to_owned(),
            weight: 1.0,
        }),
    ];
    let opts = ImportOpts { batch_size: 10, overwrite: true, dry_run: false };
    import_records(&store, &records, opts).await.unwrap();
    let expected = SourceCounts {
        memory: 1,
        entity: 1,
        edge: 1,
        project: 0,
        part: 0,
        event: 0,
    };
    let val = validate_store(&store, &expected).await.unwrap();
    assert!(val.is_clean);
    assert!(val.dangling_edges.is_empty());
}

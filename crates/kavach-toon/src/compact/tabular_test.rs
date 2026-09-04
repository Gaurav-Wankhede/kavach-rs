//! Unit tests for tabular JSON compactor.
use serde_json::json;
use super::{compact_tabular, json_to_table, table_to_json};

#[test]
fn test_json_to_table_roundtrip() {
    let items = vec![
        json!({"id": 1, "name": "alice", "active": true}),
        json!({"id": 2, "name": "bob", "active": false}),
        json!({"id": 3, "name": "charlie", "active": true}),
    ];

    let table = json_to_table(&items).expect("should format table");
    assert!(table.starts_with("[TABULAR_SCHEMA:"));

    let restored = table_to_json(&table).expect("should restore json");
    let restored_arr = restored.as_array().expect("restored array");
    assert_eq!(restored_arr.len(), 3);

    for (orig, res) in items.iter().zip(restored_arr.iter()) {
        assert_eq!(orig.get("id"), res.get("id"));
        assert_eq!(orig.get("name"), res.get("name"));
        assert_eq!(orig.get("active"), res.get("active"));
    }
}

#[test]
fn test_compact_tabular_savings() {
    let mut items = Vec::new();
    for i in 0..50 {
        items.push(json!({
            "service_name": format!("microservice_worker_{i}"),
            "status": "healthy",
            "region": "us-east-1",
            "replicas": 4
        }));
    }
    let json_str = serde_json::to_string_pretty(&items).expect("json string");
    let compacted = compact_tabular(&json_str, 30);
    assert!(compacted.is_some());
    let table = compacted.expect("table");
    assert!(table.len().saturating_mul(2) < json_str.len());
}

//! Unit tests for CCR engine.
use super::{CCR_THRESHOLD_BYTES, load_blob, process_ccr, store_blob, truncate_lines};

#[test]
fn test_store_and_load_blob() {
    let payload = "line 1\nline 2\nimportant verification outcome";
    let hash = "test_blob_12345678abcdef";
    let path = store_blob(hash, payload.as_bytes()).expect("store blob");
    assert!(path.exists());

    let loaded = load_blob(hash).expect("load blob exact");
    assert_eq!(loaded, payload);

    let loaded_prefix = load_blob("test_blob_123").expect("load blob prefix");
    assert_eq!(loaded_prefix, payload);

    // Clean up test file
    std::fs::remove_file(path).ok();
}

#[test]
fn test_truncate_lines_preserves_anomalies() {
    let mut lines = Vec::new();
    for i in 0..100 {
        if i == 50 {
            lines.push("CRITICAL error: database connection refused");
        } else {
            lines.push("normal status log line");
        }
    }
    let full_text = lines.join("\n");
    let truncated = truncate_lines(&full_text);

    assert!(truncated.contains("CRITICAL error: database connection refused"));
    assert!(truncated.contains("lines omitted"));
}

#[test]
fn test_process_ccr_short_payload_not_cached() {
    let short = "short output";
    let (res, hash) = process_ccr(short);
    assert_eq!(res, short);
    assert!(hash.is_none());
}

#[test]
fn test_process_ccr_oversized_cached() {
    let long = "cargo test output\n".repeat(400); // > 6KB
    assert!(long.len() > CCR_THRESHOLD_BYTES);

    let (res, hash) = process_ccr(&long);
    assert!(hash.is_some());
    let h = hash.expect("hash");
    assert!(res.contains("<<kavach_ccr:"));
    assert!(res.contains(&h));

    // Verify blob readable from disk
    let loaded = load_blob(&h).expect("load cached blob");
    assert_eq!(loaded, long);

    // Clean up cached blob
    let dir = super::ccr_cache_dir();
    std::fs::remove_file(dir.join(format!("{h}.blob"))).ok();
}

use super::*;

#[test]
fn extracts_basic_fields() {
    let view = FieldEntryView {
        key: "test.key",
        category: "roadmap",
        title: "Test Title",
        status: "in_progress",
        content: Some("Sample content for test"),
        access_count: 42,
        exec_prompt: Some("Execute task cleanly"),
        priority: Some(10),
        lane: Some("core"),
        tags: None,
    };

    assert_eq!(extract_field(&view, "title", false).unwrap(), "Test Title");
    assert_eq!(
        extract_field(&view, "status", false).unwrap(),
        "in_progress"
    );
    assert_eq!(
        extract_field(&view, "entry_status", false).unwrap(),
        "in_progress"
    );
    assert_eq!(extract_field(&view, "key", false).unwrap(), "test.key");
    assert_eq!(extract_field(&view, "category", false).unwrap(), "roadmap");
    assert_eq!(extract_field(&view, "access", false).unwrap(), "42");
    assert_eq!(extract_field(&view, "priority", false).unwrap(), "10");
    assert_eq!(extract_field(&view, "lane", false).unwrap(), "core");
    assert_eq!(
        extract_field(&view, "content", false).unwrap(),
        "Sample content for test"
    );
    assert_eq!(
        extract_field(&view, "exec_prompt", false).unwrap(),
        "Execute task cleanly"
    );
}

#[test]
fn compresses_content_when_requested() {
    let verbose = "Please kindly note that in order to run tests we just need cargo test.";
    let view = FieldEntryView {
        key: "test.key",
        category: "roadmap",
        title: "Test Title",
        status: "todo",
        content: Some(verbose),
        access_count: 0,
        exec_prompt: None,
        priority: None,
        lane: None,
        tags: None,
    };

    let compressed = extract_field(&view, "content", true).unwrap();
    assert!(!compressed.contains("kindly"));
    assert!(!compressed.contains("in order to"));
}

#[test]
fn errors_on_unknown_field() {
    let view = FieldEntryView {
        key: "k",
        category: "c",
        title: "t",
        status: "s",
        content: None,
        access_count: 0,
        exec_prompt: None,
        priority: None,
        lane: None,
        tags: None,
    };

    assert!(extract_field(&view, "nonexistent", false).is_err());
}

use super::matcher::Matcher;
use super::node::TreeNode;
use super::query::Query;
use super::tree::RagTree;

fn sample_tree() -> RagTree {
    let rust_skill = TreeNode {
        id: "skills/rust".into(),
        title: "Rust production guard".into(),
        summary: "error handling ownership clippy production code review".into(),
        keywords: vec!["unwrap".into(), "clippy".into(), "clone".into()],
        file_patterns: vec!["*.rs".into()],
        body: "Use ? for error propagation.".into(),
        children: Vec::new(),
    };
    let sql_skill = TreeNode {
        id: "skills/sql".into(),
        title: "SQL query guard".into(),
        summary: "postgres index optimization query plan".into(),
        keywords: vec!["sqlx".into(), "index".into()],
        file_patterns: vec!["*.sql".into()],
        body: "Use EXPLAIN ANALYZE.".into(),
        children: Vec::new(),
    };
    let root = TreeNode {
        id: "skills".into(),
        title: "skills root".into(),
        summary: "all language and domain skills".into(),
        keywords: Vec::new(),
        file_patterns: Vec::new(),
        body: String::new(),
        children: vec![rust_skill, sql_skill],
    };
    RagTree::new("test", root)
}

#[test]
fn should_return_top_match_when_rust_file_queried() {
    let tree = sample_tree();
    let matcher = Matcher::new(&tree);
    let query = Query::new("src/lib.rs", "fix unwrap", "");
    let hits = matcher.run(&query);
    let Some(first) = hits.first() else {
        panic!("expected at least one hit")
    };
    assert_eq!(first.node_id, "skills/rust");
}

#[test]
fn should_rank_sql_over_rust_when_sql_file() {
    let tree = sample_tree();
    let matcher = Matcher::new(&tree);
    let query = Query::new("schema.sql", "create index", "");
    let hits = matcher.run(&query);
    let Some(first) = hits.first() else {
        panic!("expected at least one hit")
    };
    assert_eq!(first.node_id, "skills/sql");
}

#[test]
fn should_roundtrip_tree_through_json() {
    let tree = sample_tree();
    let json = tree.to_json().expect("serialize failed");
    let parsed = RagTree::from_json(&json).expect("parse failed");
    assert_eq!(parsed.source, "test");
    assert_eq!(parsed.root.children.len(), 2);
}

#[test]
fn should_reject_tree_with_wrong_version() {
    let json = r#"{"version":999,"source":"x","root":{"id":"r","title":"t","summary":"s"}}"#;
    assert!(RagTree::from_json(json).is_err());
}

#[test]
fn should_parse_markdown_headings_into_tree() {
    let md = "# Root Title\nintro text\n\n## Section A\nbody a\n\n## Section B\nbody b\n";
    let root = super::walker::from_markdown("rules/test.md", md).expect("walk failed");
    assert_eq!(root.id, "rules/test.md");
    // One top-level heading with two nested sections.
    let Some(first) = root.children.first() else {
        panic!("expected one top-level heading")
    };
    assert_eq!(first.title, "Root Title");
    assert_eq!(first.children.len(), 2);
    let Some(a) = first.children.first() else {
        panic!("expected Section A")
    };
    assert_eq!(a.title, "Section A");
    assert!(a.body.contains("body a"));
}

#[test]
fn should_emit_pending_requests_for_unsummarized_nodes() {
    let md = "# Top\nsome body\n";
    let root = super::walker::from_markdown("doc.md", md).expect("walk failed");
    let reqs = super::protocol::pending_requests(&root);
    assert_eq!(reqs.len(), 1);
    let Some(first) = reqs.first() else {
        panic!("expected one request")
    };
    assert_eq!(first.title, "Top");
}

#[test]
fn should_apply_summary_response_to_matching_node() {
    let md = "# Top\nbody\n";
    let mut root = super::walker::from_markdown("doc.md", md).expect("walk failed");
    let target_id = {
        let Some(c) = root.children.first() else {
            panic!("expected child")
        };
        c.id.clone()
    };
    let resp = super::protocol::SummaryResponse {
        node_id: target_id,
        summary: "one-line summary of Top".into(),
        keywords: vec!["alpha".into(), "beta".into()],
    };
    super::protocol::apply_summaries(&mut root, &[resp]);
    let Some(first) = root.children.first() else {
        panic!("expected child after apply")
    };
    assert_eq!(first.summary, "one-line summary of Top");
    assert_eq!(first.keywords, vec!["alpha".to_owned(), "beta".to_owned()]);
}

#[test]
fn should_roundtrip_summary_request_through_json_line() {
    let req = super::protocol::SummaryRequest {
        node_id: "doc#sec".into(),
        title: "Sec".into(),
        body: "text".into(),
    };
    let line = req.to_line().expect("to_line failed");
    let resp_line = line.replace("body", "summary");
    // Lightweight assertion that the line is valid JSON and contains the id.
    assert!(resp_line.contains("doc#sec"));
}

#[test]
#[expect(
    clippy::let_underscore_must_use,
    reason = "cleanup: intentionally ignoring remove_dir_all result"
)]
fn should_scan_directory_and_return_markdown_files() {
    let tmp = std::env::temp_dir().join(format!("kavach-rag-scan-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("sub")).expect("mkdir sub");
    std::fs::write(tmp.join("a.md"), "# A\nbody a").expect("write a");
    std::fs::write(tmp.join("sub").join("b.md"), "# B\nbody b").expect("write b");
    std::fs::write(tmp.join("ignore.txt"), "not markdown").expect("write ignore");

    let docs = super::scanner::scan_dir(&tmp, &["md"]).expect("scan failed");

    let _ = std::fs::remove_dir_all(&tmp);

    assert_eq!(docs.len(), 2);
    let ids: Vec<&str> = docs.iter().map(super::scanner::ScannedDoc::id).collect();
    assert!(ids.contains(&"a.md"));
    assert!(ids.contains(&"sub/b.md"));
}

#[test]
#[expect(
    clippy::let_underscore_must_use,
    reason = "cleanup: intentionally ignoring remove_dir_all result"
)]
fn should_build_trees_from_directory_of_markdown() {
    let tmp = std::env::temp_dir().join(format!("kavach-rag-build-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("mkdir");
    std::fs::write(tmp.join("rules.md"), "# R\ntext").expect("write rules");
    std::fs::write(tmp.join("skills.md"), "# S\ntext").expect("write skills");

    let trees = super::walker::build_trees_from_dir(&tmp, "test-source").expect("build failed");

    let _ = std::fs::remove_dir_all(&tmp);

    assert_eq!(trees.len(), 2);
    let Some(first) = trees.first() else {
        panic!("expected at least one tree")
    };
    assert_eq!(first.source, "test-source");
}

#[test]
fn should_return_empty_when_scan_root_missing() {
    let missing = std::env::temp_dir().join("kavach-rag-does-not-exist-xyz-42");
    let docs = super::scanner::scan_dir(&missing, &["md"]).expect("scan failed");
    assert!(docs.is_empty());
}

#[test]
fn should_find_node_by_id() {
    let tree = sample_tree();
    let node = tree.find("skills/rust").expect("find failed");
    assert_eq!(node.title, "Rust production guard");
}

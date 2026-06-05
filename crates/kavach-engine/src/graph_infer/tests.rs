//! Inference rule coverage: mention emits, short/boundary/empty cases skip.
use super::{InferRow, infer_relationships};

fn row(key: &str, content: &str) -> InferRow {
    InferRow {
        project_slug: String::from("p"),
        category: String::from("roadmap"),
        entry_key: String::from(key),
        content: String::from(content),
    }
}

#[test]
fn content_mention_emits_references() {
    let rows = vec![
        row("alpha-fix", ""),
        row("beta-work", "depends on alpha-fix completion"),
    ];
    let edges = infer_relationships(&rows);
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].rel, "references");
    assert!(edges[0].target_qname.ends_with("alpha-fix"));
    assert!(edges[0].source_qname.ends_with("beta-work"));
}

#[test]
fn short_key_skipped() {
    let rows = vec![row("ab", ""), row("cd", "talking about ab here")];
    let edges = infer_relationships(&rows);
    assert!(edges.is_empty());
}

#[test]
fn word_boundary_required() {
    let rows = vec![row("alpha-fix", ""), row("other", "alpha-fixed")];
    let edges = infer_relationships(&rows);
    assert!(edges.is_empty());
}

#[test]
fn empty_input_empty_output() {
    let edges = infer_relationships(&[]);
    assert!(edges.is_empty());
}

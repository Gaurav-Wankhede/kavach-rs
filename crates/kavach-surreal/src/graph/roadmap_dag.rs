mod fetch;
mod format;
mod model;
mod render;

pub use fetch::fetch;
pub use model::{DagEdge, DagNode, RoadmapDag, TopoOrder};

#[cfg(test)]
mod decision_mermaid_tests {
    use super::model::{DagEdge, DagNode, RoadmapDag};

    fn node(id: &str, title: &str, status: &str, cat: &str) -> DagNode {
        DagNode {
            id: id.to_owned(),
            entry_key: id.rsplit('/').next().unwrap_or(id).to_owned(),
            title: title.to_owned(),
            entry_status: status.to_owned(),
            category: cat.to_owned(),
        }
    }

    fn sample() -> RoadmapDag {
        RoadmapDag {
            nodes: vec![
                node(
                    "p/decision/paseto",
                    "paseto over jwt",
                    "verified",
                    "decision",
                ),
                node(
                    "p/decision/httpsig",
                    "httpsig every call",
                    "verified",
                    "decision",
                ),
                node(
                    "p/decision/scylla",
                    "scylla displaces redis",
                    "todo",
                    "decision",
                ),
                node("p/research/x", "noise", "active", "research"),
            ],
            edges: vec![
                DagEdge {
                    source: "p/decision/paseto".into(),
                    target: "p/decision/httpsig".into(),
                    rel: "depends_on".into(),
                },
                DagEdge {
                    source: "p/decision/httpsig".into(),
                    target: "p/decision/scylla".into(),
                    rel: "supersedes".into(),
                },
            ],
        }
    }

    #[test]
    fn renders_status_styled_graph_td() {
        let m = sample().decision_mermaid(&[], 8).expect("non-empty");
        assert!(m.starts_with("graph TD\n"), "{m}");
        assert!(m.contains(":::done"), "verified ⇒ done class: {m}");
        assert!(m.contains(":::open"), "todo ⇒ open class: {m}");
        assert!(m.contains(" --> "), "{m}");
        assert!(m.contains("-.retires.-> "), "{m}");
        assert!(!m.contains("noise"), "research excluded: {m}");
    }

    #[test]
    fn focus_filter_restricts_nodes() {
        let m = sample()
            .decision_mermaid(&["paseto".to_owned()], 8)
            .expect("non-empty");
        assert!(m.contains("paseto"), "{m}");
        assert!(!m.contains("scylla"), "out-of-focus excluded: {m}");
    }

    #[test]
    fn no_decision_nodes_yields_none() {
        let only_research = RoadmapDag {
            nodes: vec![node("p/research/x", "noise", "active", "research")],
            edges: vec![],
        };
        assert!(only_research.decision_mermaid(&[], 8).is_none());
    }

    #[test]
    fn max_nodes_caps_output() {
        let m = sample().decision_mermaid(&[], 1).expect("non-empty");
        let node_lines = m.lines().filter(|l| l.contains(":::")).count();
        assert_eq!(node_lines, 1, "{m}");
    }

    fn pattern_sample() -> RoadmapDag {
        RoadmapDag {
            nodes: vec![
                node(
                    "p/pattern/dioxus-0.8",
                    "dioxus-0.8 location via BFF",
                    "todo",
                    "pattern",
                ),
                node(
                    "p/pattern/dioxus-0.7",
                    "dioxus-0.7 web-sys gap",
                    "verified",
                    "pattern",
                ),
                node("p/decision/x", "noise", "verified", "decision"),
            ],
            edges: vec![DagEdge {
                source: "p/pattern/dioxus-0.8".into(),
                target: "p/pattern/dioxus-0.7".into(),
                rel: "supersedes".into(),
            }],
        }
    }

    #[test]
    fn pattern_dag_renders_supersession_with_trust_tag() {
        let m = pattern_sample()
            .pattern_dag_mermaid(&[], 8)
            .expect("non-empty");
        assert!(m.starts_with("graph TD\n"), "{m}");
        assert!(m.contains("(fresh)"), "unsoaked pattern tagged: {m}");
        assert!(m.contains("-.retires.-> "), "{m}");
        assert!(!m.contains("noise"), "non-pattern excluded: {m}");
    }

    #[test]
    fn pattern_dag_no_pattern_nodes_yields_none() {
        let only_decision = RoadmapDag {
            nodes: vec![node("p/decision/x", "noise", "verified", "decision")],
            edges: vec![],
        };
        assert!(only_decision.pattern_dag_mermaid(&[], 8).is_none());
    }

    #[test]
    fn pattern_dag_focus_filter_restricts_nodes() {
        let m = pattern_sample()
            .pattern_dag_mermaid(&["dioxus-0.8".to_owned()], 8)
            .expect("non-empty");
        assert!(m.contains("dioxus-0.8"), "{m}");
        assert!(!m.contains("dioxus-0.7"), "out-of-focus excluded: {m}");
    }

    #[test]
    fn pattern_dag_focus_with_no_match_yields_none() {
        assert!(
            pattern_sample()
                .pattern_dag_mermaid(&["unrelated-key".to_owned()], 8)
                .is_none()
        );
    }

    #[test]
    fn retired_patterns_returns_target_and_replacement() {
        let r = pattern_sample().retired_patterns();
        assert_eq!(r.len(), 1, "{r:?}");
        assert_eq!(r[0].0, "dioxus-0.7 web-sys gap");
        assert_eq!(r[0].1, "dioxus-0.8 location via BFF");
    }

    #[test]
    fn retired_patterns_empty_without_supersedes() {
        let no_edge = RoadmapDag {
            nodes: vec![node("p/pattern/x", "p", "verified", "pattern")],
            edges: vec![],
        };
        assert!(no_edge.retired_patterns().is_empty());
    }

    #[tokio::test]
    async fn fetch_surfaces_supersedes_edge_as_retired_pair() {
        use crate::{
            apply_schema, open_memory, project_register, upsert_entry_full, upsert_relationships,
        };

        let db = open_memory().await.expect("open mem db");
        apply_schema(&db).await.expect("schema");
        let proj = project_register(&db, "proofproj", "Proof", "/tmp", None)
            .await
            .expect("register project");

        for (key, title) in [
            ("old-pat", "old pattern: web-sys gap"),
            ("new-pat", "new pattern: router hook"),
        ] {
            let qn = format!("proofproj/pattern/{key}");
            upsert_entry_full()
                .db(&db)
                .category("pattern")
                .project_id(&proj)
                .entry_key(key)
                .title(title)
                .content("c")
                .event_source("test")
                .qualified_name(&qn)
                .references(&[])
                .build_for_call()
                .await
                .expect("upsert pattern");
        }

        let n = upsert_relationships(
            &db,
            "proofproj/pattern/new-pat",
            &[(
                "supersedes".to_owned(),
                "proofproj/pattern/old-pat".to_owned(),
            )],
        )
        .await
        .expect("relate");
        assert_eq!(n, 1, "one supersedes edge written");

        let dag = super::fetch(&db, "proofproj").await.expect("fetch");
        assert_eq!(dag.nodes.len(), 2, "two pattern nodes: {:?}", dag.nodes);
        assert!(
            dag.edges.iter().any(|e| e.rel == "supersedes"),
            "supersedes edge survived into dag: {:?}",
            dag.edges
        );
        let retired = dag.retired_patterns();
        assert_eq!(retired.len(), 1, "exactly one retired pair: {retired:?}");
        assert_eq!(retired[0].0, "old pattern: web-sys gap", "retired title");
        assert_eq!(
            retired[0].1, "new pattern: router hook",
            "replacement title"
        );
    }
}

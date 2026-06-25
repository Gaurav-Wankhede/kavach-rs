use super::fetch::fetch_flow;
use super::model::{FlowDag, FlowEdgeInput, FlowSpec, FlowStep, FlowStepInput};
use super::shape::NodeShape;
use super::upsert::upsert_flow;

    fn sample() -> FlowDag {
        FlowDag {
            flow_key: "auth".to_owned(),
            flow_title: "Auth Flow".to_owned(),
            steps: vec![
                FlowStep {
                    step_id: "validate".to_owned(),
                    label: "Validate input".to_owned(),
                    shape: NodeShape::Rect,
                    description: None,
                },
                FlowStep {
                    step_id: "issue-token".to_owned(),
                    label: "Issue \"token\"".to_owned(),
                    shape: NodeShape::Diamond,
                    description: Some("mint PASETO".to_owned()),
                },
            ],
            edges: vec![("validate".to_owned(), "issue-token".to_owned())],
            raw_mermaid: None,
        }
    }

    #[test]
    fn mermaid_sanitizes_ids_and_escapes_labels() {
        let m = sample().to_mermaid();
        assert!(m.starts_with("flowchart TD\n"), "header: {m}");
        assert!(m.contains("issue_token{\"Issue &quot;token&quot;\"}"), "{m}");
        assert!(m.contains("validate[\"Validate input\"]"), "{m}");
        assert!(m.contains("validate --> issue_token"), "{m}");
    }

    #[test]
    fn detect_cycle_flags_a_cycle() {
        let mut dag = sample();
        dag.edges.push(("issue-token".to_owned(), "validate".to_owned()));
        let cycle = dag.detect_cycle().expect("should detect cycle");
        assert_eq!(cycle.len(), 2, "both steps are in the cycle: {cycle:?}");
    }

    #[test]
    fn detect_cycle_accepts_a_dag() {
        assert!(sample().detect_cycle().is_none());
    }

    #[test]
    fn shape_roundtrips_through_str() {
        for s in [
            NodeShape::Rect,
            NodeShape::Round,
            NodeShape::Stadium,
            NodeShape::Diamond,
            NodeShape::Circle,
        ] {
            assert_eq!(NodeShape::parse(s.as_str()), s);
        }
        assert_eq!(NodeShape::parse("nonsense"), NodeShape::Rect);
    }

    fn spec() -> FlowSpec {
        FlowSpec {
            flow_key: "auth".to_owned(),
            flow_title: "Auth Flow".to_owned(),
            steps: vec![
                FlowStepInput {
                    step_id: "validate".to_owned(),
                    label: "Validate input".to_owned(),
                    shape: Some("rect".to_owned()),
                    description: None,
                },
                FlowStepInput {
                    step_id: "issue".to_owned(),
                    label: "Issue token".to_owned(),
                    shape: Some("diamond".to_owned()),
                    description: Some("mint PASETO".to_owned()),
                },
            ],
            edges: vec![FlowEdgeInput {
                from: "validate".to_owned(),
                to: "issue".to_owned(),
            }],
            raw_mermaid: None,
        }
    }

    async fn db_with_project() -> surrealdb::Surreal<surrealdb::engine::any::Any> {
        let db = crate::open_memory().await.expect("open in-memory db");
        crate::apply_schema(&db).await.expect("schema");
        db.query("CREATE project SET slug = 'p', display = 'P', workdir = '/tmp'")
            .await
            .expect("create project");
        db
    }

    fn keyset(dag: &FlowDag) -> (Vec<String>, Vec<(String, String)>) {
        let mut steps: Vec<String> = dag.steps.iter().map(|s| s.step_id.clone()).collect();
        steps.sort();
        let mut edges = dag.edges.clone();
        edges.sort();
        (steps, edges)
    }

    #[tokio::test]
    async fn roundtrip_ingest_render_reingest_is_stable() {
        let db = db_with_project().await;
        upsert_flow(&db, "p", &spec()).await.expect("first upsert");
        let dag1 = fetch_flow(&db, "p", "auth").await.expect("fetch 1");
        let m = dag1.to_mermaid();
        assert!(m.contains("validate[\"Validate input\"]"), "{m}");
        assert!(m.contains("issue{\"Issue token\"}"), "{m}");
        assert!(m.contains("validate --> issue"), "{m}");

        upsert_flow(&db, "p", &spec()).await.expect("second upsert");
        let dag2 = fetch_flow(&db, "p", "auth").await.expect("fetch 2");
        assert_eq!(keyset(&dag1), keyset(&dag2), "round-trip is stable");
        assert_eq!(dag2.steps.len(), 2, "no duplicate steps on re-ingest");
    }

    #[tokio::test]
    async fn upsert_rejects_a_cycle_before_writing() {
        let db = db_with_project().await;
        let mut s = spec();
        s.edges.push(FlowEdgeInput {
            from: "issue".to_owned(),
            to: "validate".to_owned(),
        });
        let err = upsert_flow(&db, "p", &s).await.expect_err("cycle rejected");
        assert!(format!("{err}").contains("cycle"), "{err}");
        assert!(fetch_flow(&db, "p", "auth").await.is_err(), "no anchor written");
    }

    #[tokio::test]
    async fn upsert_rejects_edge_to_unknown_step() {
        let db = db_with_project().await;
        let mut s = spec();
        s.edges.push(FlowEdgeInput {
            from: "validate".to_owned(),
            to: "ghost".to_owned(),
        });
        let err = upsert_flow(&db, "p", &s).await.expect_err("unknown step");
        assert!(format!("{err}").contains("unknown step"), "{err}");
    }

    #[tokio::test]
    async fn upsert_rejects_unregistered_project() {
        let db = crate::open_memory().await.expect("open in-memory db");
        crate::apply_schema(&db).await.expect("schema");
        let err = upsert_flow(&db, "nope", &spec())
            .await
            .expect_err("unregistered project");
        assert!(format!("{err}").contains("not registered"), "{err}");
    }

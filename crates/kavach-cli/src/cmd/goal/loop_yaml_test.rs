// Round-trip + default-fill proofs for the goal-loop data model.
// THE ORACLE: a GoalLoopYaml serializes to YAML and parses back to an equal
// value, and an absent `harness`/`loop_limits` fills its default.
use super::{GoalLoopYaml, Harness, OnMaxAttempts};
use std::path::Path;

#[test]
fn round_trips() {
    let g = GoalLoopYaml::test_exit_code(
        "goal-paseto-introspect",
        "Wire paseto.rs -> introspect end-to-end",
        "cargo nextest run -p kavach-rpc introspect",
    );
    let yaml = g.to_yaml().expect("serialize");
    let back = GoalLoopYaml::from_yaml(&yaml).expect("parse");
    assert_eq!(g, back);
}

#[test]
fn oracle_tag_is_kebab_case() {
    let g = GoalLoopYaml::test_exit_code("g", "i", "true");
    let yaml = g.to_yaml().expect("serialize");
    assert!(yaml.contains("type: test-exit-code"), "got:\n{yaml}");
}

#[test]
fn harness_defaults_to_loop_until_done_when_absent() {
    // Pre-enhancement YAML has no `harness` key — must still parse, as Pattern 6.
    let yaml = "goal_id: g\nintent: i\noracle:\n  type: test-exit-code\n  check: 'true'\n";
    let g = GoalLoopYaml::from_yaml(yaml).expect("parse without harness");
    assert_eq!(g.harness, Harness::LoopUntilDone);
}

#[test]
fn harness_round_trips_each_variant() {
    let variants = [
        Harness::ClassifyAct {
            routes: vec!["fixer".into(), "answerer".into()],
        },
        Harness::FanOutSynthesize { shards: 4 },
        Harness::WorkerCritic { critics: 3 },
        Harness::GenerateFilter { candidates: 8 },
        Harness::PairwiseTournament { competitors: 4 },
        Harness::LoopUntilDone,
    ];
    for h in variants {
        let mut g = GoalLoopYaml::test_exit_code("g", "i", "true");
        g.harness = h.clone();
        let back = GoalLoopYaml::from_yaml(&g.to_yaml().expect("ser")).expect("de");
        assert_eq!(back.harness, h, "round-trip mismatch for {h:?}");
    }
}

#[test]
fn harness_tag_is_kebab_case() {
    let mut g = GoalLoopYaml::test_exit_code("g", "i", "true");
    g.harness = Harness::WorkerCritic { critics: 3 };
    let yaml = g.to_yaml().expect("serialize");
    assert!(yaml.contains("pattern: worker-critic"), "got:\n{yaml}");
}

#[test]
fn defaults_fill_loop_limits_when_absent() {
    let yaml = "goal_id: g\nintent: i\noracle:\n  type: predicate\n  check: 'file exists'\n";
    let g = GoalLoopYaml::from_yaml(yaml).expect("parse with defaulted loop_limits");
    assert_eq!(g.loop_limits.max_attempts, 3);
    assert_eq!(g.loop_limits.on_max_attempts, OnMaxAttempts::Escalate);
}

#[test]
fn loop_path_is_under_kavach_goals() {
    let g = GoalLoopYaml::test_exit_code("my-goal", "i", "true");
    assert_eq!(
        g.loop_path(),
        Path::new(".kavach")
            .join("goals")
            .join("my-goal")
            .join("loop.yaml")
    );
}

#[test]
fn emit_writes_file_and_round_trips_from_disk() {
    let dir = std::env::temp_dir().join(format!("kavach-loopyaml-{}", std::process::id()));
    let g = GoalLoopYaml::test_exit_code("emit-test", "i", "cargo test");
    let rel = g.emit(&dir).expect("emit");
    let abs = dir.join(&rel);
    let read = std::fs::read_to_string(&abs).expect("read back");
    let back = GoalLoopYaml::from_yaml(&read).expect("parse from disk");
    assert_eq!(g, back);
    drop(std::fs::remove_dir_all(&dir));
}

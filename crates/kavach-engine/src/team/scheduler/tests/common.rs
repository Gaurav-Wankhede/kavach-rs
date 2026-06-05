//! Shared test fixtures: DAG node/edge builders, a scheduler ctor, and the
//! scripted spawners used across the plan/dispatch/multitick test modules.
use std::cell::RefCell;

use kavach_surreal::graph::roadmap_dag::{DagEdge, DagNode};

use super::super::{DagScheduler, Spawner, SpawnerKind};
use crate::error::EngineError;

pub(super) fn node(id: &str, status: &str) -> DagNode {
    DagNode {
        id: id.into(),
        entry_key: id.into(),
        title: format!("task {id}"),
        entry_status: status.into(),
        category: "roadmap".into(),
    }
}

/// edge: prereq must finish before dependent (source=prereq, target=dependent).
pub(super) fn dep(prereq: &str, dependent: &str) -> DagEdge {
    DagEdge {
        source: prereq.into(),
        target: dependent.into(),
        rel: "depends_on".into(),
    }
}

pub(super) fn scheduler(cap: usize) -> DagScheduler {
    DagScheduler::with_cap(cap, SpawnerKind::CcTeams)
}

/// Records each spawned key; never fails. For plan/dispatch batch-count checks.
pub(super) struct CountSpawner(pub RefCell<Vec<String>>);

impl Spawner for CountSpawner {
    fn spawn(&self, task_key: &str, _title: &str) -> Result<String, EngineError> {
        self.0.borrow_mut().push(task_key.to_owned());
        Ok(format!("mate-{task_key}"))
    }
}

/// Records calls; optionally fails at a given call index. For multi-tick + the
/// mid-batch failure-propagation tests.
pub(super) struct ScriptSpawner {
    calls: RefCell<Vec<String>>,
    fail_at: Option<usize>,
}

impl ScriptSpawner {
    pub(super) fn new(fail_at: Option<usize>) -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
            fail_at,
        }
    }

    pub(super) fn call_log(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }
}

impl Spawner for ScriptSpawner {
    fn spawn(&self, task_key: &str, _title: &str) -> Result<String, EngineError> {
        let mut log = self.calls.borrow_mut();
        let call_idx = log.len();
        log.push(task_key.to_owned());
        if self.fail_at == Some(call_idx) {
            return Err(EngineError::Session("injected spawner failure".into()));
        }
        Ok(format!("mate-{task_key}"))
    }
}

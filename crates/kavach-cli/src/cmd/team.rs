// kavach team — DAG-aware parallel Team auto-dispatch over the roadmap.
// SOURCE: roadmap.unit.dag-parallel-dispatch.
//
// `kavach team dispatch --project X` resolves the roadmap dependency DAG, then
// fans CC Team agents across the ready wavefront: independent tasks dispatch
// concurrently; a blocked task stays out of the batch until its prerequisite
// is done/verified. A dependency cycle is rejected (fail closed) naming the keys.

use std::cell::RefCell;

use clap::{Args, Subcommand};

use kavach_engine::{
    role_for_title, DagScheduler, EngineError, RewardRouter, RolePool, Spawner, SpawnerKind,
    TeamDispatchError, VendorRequest,
};
use kavach_patterns::bandit_log::Reward;
use kavach_surreal::graph::roadmap_dag::RoadmapDag;

use crate::cmd::io_safe::{IoExit, ewrite_or_exit, into_exit_code, print_or_exit};

#[derive(Args)]
#[command(
    about = "DAG-aware parallel Team auto-dispatch over the roadmap",
    long_about = "Resolves roadmap dependency DAG, fans CC Team agents across the ready \
wavefront. Independent tasks dispatch concurrently; blocked tasks wait for prerequisites. \
Cycles fail closed with named keys.",
    after_help = "EXAMPLES:\n  kavach team dispatch --project nicole-carpenter --dry-run\n  \
kavach team dispatch --project P --max-parallel 4 --spawner cc-teams\n\nWHEN: Multi-unit \
parallel implementation with explicit depends_on edges."
)]
pub(crate) struct TeamArgs {
    #[command(subcommand)]
    pub(crate) action: TeamAction,
}

#[derive(Subcommand)]
pub(crate) enum TeamAction {
    /// Resolve the roadmap DAG and dispatch the ready wavefront in parallel.
    Dispatch {
        /// Project slug whose roadmap DAG to schedule.
        #[arg(long)]
        project: String,
        /// Concurrency cap (clamped to [1,16]); default = min(16, cores-2).
        #[arg(long)]
        max_parallel: Option<usize>,
        /// Print the topological wavefront only; spawn nothing.
        #[arg(long)]
        dry_run: bool,
        /// Spawn backend: cc-teams (default) | workflow.
        #[arg(long, default_value = "cc-teams")]
        spawner: String,
    },
}

pub(crate) fn run(args: TeamArgs) -> i32 {
    match args.action {
        TeamAction::Dispatch {
            project,
            max_parallel,
            dry_run,
            spawner,
        } => {
            let runtime = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => return report_err(&format!("team dispatch: tokio runtime: {e}")),
            };
            match runtime.block_on(dispatch(&project, max_parallel, dry_run, &spawner)) {
                Ok(code) => code,
                Err(io) => into_exit_code(io),
            }
        }
    }
}

/// Best-effort stderr report → exit 1. Used for non-IO failures where stdout
/// is unaffected; an IO failure on the report itself collapses to `into_exit_code`.
fn report_err(msg: &str) -> i32 {
    ewrite_or_exit(msg).map_or_else(into_exit_code, |()| 1)
}

fn parse_spawner(s: &str) -> SpawnerKind {
    match s {
        "workflow" => SpawnerKind::Workflow,
        _ => SpawnerKind::CcTeams,
    }
}

async fn dispatch(
    project: &str,
    max_parallel: Option<usize>,
    dry_run: bool,
    spawner: &str,
) -> Result<i32, IoExit> {
    let db = match kavach_surreal::open_default().await {
        Ok(db) => db,
        Err(e) => return Ok(report_err(&format!("team dispatch: open db: {e}"))),
    };
    let dag: RoadmapDag = match kavach_surreal::roadmap_dag_fetch(&db, project).await {
        Ok(dag) => dag,
        Err(e) => {
            return Ok(report_err(&format!(
                "team dispatch: fetch DAG for '{project}': {e}"
            )));
        }
    };

    let kind = parse_spawner(spawner);
    // with_cap clamps to [1,16]; None defers to the default cap via a value far
    // above the clamp ceiling so the constructor picks min(16, cores-2).
    let scheduler = DagScheduler::for_cli(max_parallel, kind);

    // First CLI tick: free-slot accounting starts at zero active teammates.
    let plan = match scheduler.plan(&dag, 0) {
        Ok(plan) => plan,
        Err(TeamDispatchError::Cycle(keys)) => {
            return Ok(report_err(&format!(
                "[DAG_CYCLE] dependency cycle (deadlock) among: {keys:?}"
            )));
        }
        Err(TeamDispatchError::Engine(e)) => {
            return Ok(report_err(&format!("team dispatch: {e}")));
        }
    };

    print_or_exit(&format!(
        "[DAG_WAVEFRONT] project={project} ready={} cap={} free={} batch={}",
        plan.ready.len(),
        scheduler.cap(),
        plan.free_slots,
        plan.batch.len(),
    ))?;
    for (i, key) in plan.ready.iter().enumerate() {
        let claimable = if i < plan.batch.len() {
            "DISPATCH"
        } else {
            "queued "
        };
        print_or_exit(&format!("  [{claimable}] {key}"))?;
    }

    if dry_run {
        print_or_exit("[DRY_RUN] no agents spawned")?;
        return Ok(0);
    }

    let sp = LogSpawner {
        io_err: RefCell::new(None),
        pool: RolePool::default(),
        project: project.to_owned(),
    };
    match scheduler.dispatch(&dag, 0, &sp) {
        Ok(names) => {
            if let Some(io) = sp.io_err.into_inner() {
                return Err(io);
            }
            print_or_exit(&format!(
                "[DISPATCHED] {} teammate(s): {names:?}",
                names.len()
            ))?;
            Ok(0)
        }
        Err(e) => Ok(report_err(&format!("team dispatch: {e}"))),
    }
}

/// Spawner that records the claim plan to stdout. Live CC-Teams / Workflow
/// launch is a host-side primitive (the harness owns teammate creation); this
/// CLI tick resolves the batch and emits the plan the host acts on. A stdout
/// failure is captured (not swallowed) and re-surfaced by the caller.
struct LogSpawner {
    io_err: RefCell<Option<IoExit>>,
    pool: RolePool,
    project: String,
    router: RefCell<RewardRouter>,
}

impl Spawner for LogSpawner {
    fn spawn(&self, task_key: &str, title: &str) -> Result<String, EngineError> {
        let role = role_for_title(title);
        let backend = self.pool.backend_for(role);
        let vendor = backend.id().to_owned();
        if let Err(io) = print_or_exit(&format!(
            "[SPAWN] task={task_key} role={role:?} vendor={vendor} title={title:?}"
        )) {
            *self.io_err.borrow_mut() = Some(io);
        }
        let req = VendorRequest::new(role, title.to_owned(), self.project.clone(), 8);
        let result = backend.dispatch(&req);
        // Conductor RL: feed the outcome back into the router so vendor/role
        // routing evolves. exit-0 = VerifiedClean, any failure = FalseDecision.
        let reward = match &result {
            Ok(out) if out.exit_code == 0 => Reward::VerifiedClean,
            _ => Reward::FalseDecision,
        };
        self.router.borrow_mut().record(&vendor, role, reward);
        let out = result?;
        Ok(format!("{vendor}:{task_key}:exit{}", out.exit_code))
    }
}

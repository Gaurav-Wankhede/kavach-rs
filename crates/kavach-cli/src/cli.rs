// hub: clap CLI entry point — Cli struct and Commands enum are intentionally here
// ARCH: see kavach db get --category decision --key arch.decision.fourteen_prefix_const_table
use clap::{Parser, Subcommand};

mod db;
mod harness_loop;
mod oversized;
mod phase;
mod rag;
mod rules;
mod security;
mod session;
mod spec;
mod tailwind_plus;
mod tasks;
mod todos;

pub(crate) use db::DbAction;
pub(crate) use harness_loop::LoopAction;
pub(crate) use oversized::{OversizedAction, OversizedFormat};
pub(crate) use phase::PhaseAction;
pub(crate) use rag::RagAction;
pub(crate) use rules::RulesAction;
pub(crate) use security::SecurityAction;
pub(crate) use session::SessionAction;
pub(crate) use spec::SpecAction;
pub(crate) use tailwind_plus::TailwindPlusAction;
pub(crate) use tasks::TasksAction;
pub(crate) use todos::TodosAction;
// PipelineAction is defined inline in this file (initializer→subagent pipeline).

/// Build version string emitted by build.rs as `<iso>+git:<sha>`.
/// Read at compile time so `kavach --version` returns the exact build identity.
pub(crate) const KAVACH_VERSION: &str = env!("KAVACH_VERSION");
pub(crate) const KAVACH_BUILD_TIMESTAMP: &str = env!("KAVACH_BUILD_TIMESTAMP");
pub(crate) const KAVACH_GIT_SHA: &str = env!("KAVACH_GIT_SHA");

#[derive(Parser)]
#[command(
    name = "kavach",
    about = "Kavach — Claude Code enforcement hooks",
    version = KAVACH_VERSION,
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Show session status and enforcement state
    Status,
    /// Run a gate hook (called by Claude Code hooks)
    Gates {
        /// Gate name (e.g. pre-write, post-write, pre-tool, post-tool, intent)
        gate_name: String,
        /// Read JSON input from stdin (hook mode)
        #[arg(long)]
        hook: bool,
        /// Verify a prompt against a gate without hook mode (dry-run)
        #[arg(long)]
        verify: Option<String>,
    },
    /// Manage session lifecycle
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
    /// Manage rules and skills
    Rules {
        #[command(subcommand)]
        action: RulesAction,
    },
    /// Manage the SurrealDB-backed memory store (projects, kanban, decisions, graph)
    Db {
        #[command(subcommand)]
        action: DbAction,
    },
    /// Run JSON-RPC 2.0 server backed by kavach-surreal (stdio or http)
    Rpc {
        /// Transport mode: stdio (default, for Claude Code) or http (random ephemeral port + lockfile)
        #[arg(long, default_value = "stdio")]
        transport: String,
        /// Apply schema on startup (idempotent)
        #[arg(long, default_value_t = true)]
        apply_schema: bool,
    },
    /// Build and query vectorless RAG trees
    Rag {
        #[command(subcommand)]
        action: RagAction,
    },
    /// Ask the advisor (Haiku executor + Opus advisor)
    Ask {
        /// Prompt to send to the advisor
        prompt: String,
        /// Maximum advisor consultations (default 3)
        #[arg(long, default_value_t = 3)]
        max_uses: u8,
    },
    /// Scan for oversized files and circular dependency clusters
    Oversized {
        #[command(subcommand)]
        action: OversizedAction,
    },
    /// Build and query the Tailwind Plus component index
    TailwindPlus {
        #[command(subcommand)]
        action: TailwindPlusAction,
    },
    /// Manage SDLC development phases (PLAN/IMPLEMENT/TEST/HARDEN)
    Phase {
        #[command(subcommand)]
        action: PhaseAction,
    },
    /// Manage specification artifacts (six-file context, auto-draft)
    Spec {
        #[command(subcommand)]
        action: SpecAction,
    },
    /// Manage autonomous execution loop (harness engineering)
    Loop {
        #[command(subcommand)]
        action: LoopAction,
    },
    /// Verify a roadmap entry: run cargo check + tests, then transition done→verified.
    /// SOURCE: 42-pattern catalog §3.5 Writer-is-not-Evaluator separation.
    Verify {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Roadmap entry key to verify
        #[arg(long)]
        key: String,
        /// Crate to test (default: workspace)
        #[arg(long)]
        crate_name: Option<String>,
    },
    /// One-shot deploy: cargo build --release, install binary to ~/.local/bin/kavach.
    /// With --bundle, also builds the Kavach.app GUI (via `dx bundle`) with the
    /// CLI embedded as a sidecar, codesigns it, installs to /Applications, and
    /// symlinks ~/.local/bin/kavach into the bundle.
    /// SOURCE: `AgentCore` CLI pattern — agentcore deploy.
    Deploy {
        /// Skip the cargo nextest step (build + install only).
        #[arg(long)]
        skip_tests: bool,
        /// Also build + install the GUI app bundle (Kavach.app + .dmg) with the
        /// CLI embedded. Requires the `dx` CLI (Dioxus 0.7). macOS only.
        #[arg(long)]
        bundle: bool,
    },
    /// Strict TS/frontend gate: detect biome/eslint/tsc, auto-fix safe rewrites,
    /// fail on any warning. Mirror of `kavach deploy` for non-Rust projects.
    /// Self-healing harness for production codes per CLAUDE.md §10.
    VerifyFrontend {
        /// Project root (must contain biome.json / eslint.config.* / tsconfig.json)
        #[arg(long)]
        path: String,
        /// Skip the test runner step (still runs auto-fix + strict lint + tsc)
        #[arg(long)]
        skip_tests: bool,
        /// Tool preference when multiple configs are present: auto | biome | eslint
        #[arg(long, default_value = "auto")]
        prefer: String,
    },
    /// Pipeline operations: plan from `AppSpec`, show status of multi-stage workflow.
    /// SOURCE: 42-pattern catalog §5.1 Agent-Skill-Command Triad — initializer→subagent.
    Pipeline {
        #[command(subcommand)]
        action: PipelineAction,
    },
    /// DeepSec-style security scanner: regex pre-filter → LLM deep analysis → report
    Security {
        #[command(subcommand)]
        action: SecurityAction,
    },
    /// Sync `kavach_todo`!() macros from source to kanban
    Todos {
        #[command(subcommand)]
        action: TodosAction,
    },
    /// Audit Claude Code's `TaskCreate` JSON store (user-global by design) and infer
    /// which project each task likely belongs to. Workaround for cross-project
    /// task-list pollution — see roadmap.unit.task-injection-project-scope.
    Tasks {
        #[command(subcommand)]
        action: TasksAction,
    },
    /// Launch the kavach desktop app (Dioxus 0.7) — visualizes projects, roadmap, kanban, decisions, knowledge graph.
    App,
    /// Unified context payload for AI agent harness — single JSON with kanban, session, phase, loop state
    Context {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Max kanban items (default: 10)
        #[arg(long, default_value_t = 10)]
        limit: usize,
        /// Filter kanban by status
        #[arg(long)]
        status: Option<String>,
        /// Filter kanban by key substring
        #[arg(long)]
        key: Option<String>,
    },
    /// Inspect / clear the K-PRI mistake ledger (auto-populated by stop-gate behavioral blocks).
    Mistake(crate::cmd::mistake::MistakeArgs),
    /// Bulk-mode: one [RCA] binds N edits in a mechanical sweep.
    /// SOURCE: roadmap.unit.kavach-bulk-mode.
    Bulk(crate::cmd::bulk::BulkArgs),
    /// Goal-mode: declare a condition for CC 2.1.139+ /goal cross-turn loops.
    /// SOURCE: roadmap.unit.kavach-goal-bridge.
    Goal(crate::cmd::goal::GoalArgs),
    /// Bg-mode: declare a CC 2.1.152+ /bg background-session intent keyed to a roadmap unit.
    /// SOURCE: roadmap.unit.kavach-bg-session · code.claude.com/docs/en/changelog 2.1.152.
    Bg(crate::cmd::bg::BgArgs),
    /// Team-mode: DAG-aware parallel auto-dispatch of CC Team agents over the
    /// roadmap. Independent tasks fan out; blocked tasks wait for prerequisites.
    /// SOURCE: roadmap.unit.dag-parallel-dispatch.
    Team(crate::cmd::team::TeamArgs),
    /// MCP (Model Context Protocol) stdio server bridging Claude Code to kavach-db.
    /// Register with: `claude mcp add kavach -- kavach mcp`.
    Mcp,
}

#[derive(Subcommand)]
pub(crate) enum PipelineAction {
    /// Read an `app_spec` entry and emit roadmap items (status=todo) from its tasks.
    Plan {
        /// Project slug
        #[arg(long)]
        project: String,
        /// `AppSpec` `entry_key` to read from `memory_entries` (`category=app_spec`)
        #[arg(long)]
        spec_key: String,
    },
    /// Show counts of {todo, `in_progress`, done, verified, blocked, deferred} for a project.
    Status {
        /// Project slug
        #[arg(long)]
        project: String,
    },
}

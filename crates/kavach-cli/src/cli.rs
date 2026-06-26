// hub: clap CLI entry point — Cli struct and Commands enum are intentionally here
// ARCH: see kavach db get --category decision --key arch.decision.fourteen_prefix_const_table
use clap::{Parser, Subcommand};

mod db;
mod harness_loop;
pub(crate) mod help_md;
mod help_stack;
pub(crate) mod help_tree;
mod oversized;
mod phase;
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
    about = "Kavach harness — hooks, SurrealDB kanban, SDLC phases, agent loop control",
    long_about = "Kavach wraps AI agent sessions with enforcement hooks, a SurrealDB \
memory store (projects, roadmap/kanban, decisions, graph), and SDLC phase gates.\n\n\
WORKFLOWS (common agent paths):\n  \
Session start:  kavach db kanban --project <slug>\n  \
                kavach db get --project <slug> --category roadmap --key <unit> --full\n  \
                kavach phase status && kavach context --project <slug>\n  \
Close a card:   kavach db status-update … --status done\n  \
                kavach verify --project <slug> --key <unit> --crate-name <crate>\n  \
                kavach db kanban-close --project <slug> --key <unit>\n  \
Health check:   kavach db kanban && kavach context (direct DB — no RPC required)\n\n\
Run `kavach <command> --help` for subcommands, flags, and examples.",
    version = KAVACH_VERSION
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Show session status, build identity, and enforcement flags.
    #[command(
        after_help = "EXAMPLES:\n  kavach status\n\nWHEN: session-start sanity check; confirms project slug and pending gates."
    )]
    Status,
    /// Launch the HTMX web UI (server-rendered) on <http://127.0.0.1>:<port>.
    #[command(
        after_help = "EXAMPLES:\n  kavach web              # serve on :7777\n  kavach web --port 8080\n\nWHEN: browse projects/roadmap/kanban/decisions/knowledge in a browser. Reads via the surreal server; run `kavach servers up` first if pages show the offline panel."
    )]
    Web {
        /// TCP port to bind on loopback (default 7777, unprivileged).
        #[arg(long, default_value_t = kavach_web::DEFAULT_PORT)]
        port: u16,
    },
    /// Start/stop/inspect the background servers (`SurrealDB` store + web UI).
    #[command(
        after_help = "EXAMPLES:\n  kavach servers up        # ensure DB + GUI running, print URL\n  kavach servers status\n  kavach servers down\n\nWHEN: bring the GUI online, or check why pages show the offline panel."
    )]
    Servers {
        #[command(subcommand)]
        action: ServersAction,
    },
    /// Run a gate hook (called by Claude Code / Cursor hooks).
    #[command(
        after_help = "EXAMPLES:\n  kavach gates stop --help          # gate purpose (no stdin)\n  echo '{\"hook_event_name\":\"Stop\",\"cwd\":\".\"}' | kavach gates stop --hook --vendor cursor\n\nWHEN: IDE hooks only. For kanban health use `kavach db kanban` or `kavach context`."
    )]
    Gates {
        /// Gate name (pre-write, post-write, pre-tool, post-tool, intent, stop, session-start, six-file-intent, pre-implementation, post-implementation, …)
        gate_name: String,
        /// Read JSON input from stdin (hook mode)
        #[arg(long)]
        hook: bool,
        /// Verify a prompt against a gate without hook mode (dry-run)
        #[arg(long)]
        verify: Option<String>,
        /// Force the harness dialect (claude-code|cursor|codex). Omit to
        /// auto-detect from the payload (falls back to `$KAVACH_HARNESS`, then
        /// Claude Code). Lets one hook command serve all three IDEs.
        #[arg(long)]
        vendor: Option<String>,
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
    /// Manage the SurrealDB-backed memory store (projects, kanban, decisions, graph).
    #[command(
        after_help = "EXAMPLES:\n  kavach db kanban --project nicole-carpenter --limit 10\n  kavach db get --project P --category roadmap --key K --full\n  kavach db write --project P --category roadmap --key K --title T --new\n\nWHEN: All kanban/roadmap truth lives here. Prefer `db kanban` over RPC for health checks."
    )]
    Db {
        #[command(subcommand)]
        action: DbAction,
    },
    /// Install Kavach's OFFICIAL hook config into a native tool (CC/Cursor/Codex/…).
    #[command(
        after_help = "EXAMPLES:\n  kavach install --vendor all --dry-run\n  kavach install --vendor cursor\n\nWHEN: one-time onboarding — makes each tool load Kavach via its OWN official hook mechanism. Backs up + idempotent."
    )]
    Install {
        /// Which tool(s): cc | cursor | codex | gemini | pi | all.
        #[arg(long)]
        vendor: String,
        /// Preview the would-be action without writing any file.
        #[arg(long)]
        dry_run: bool,
    },
    /// Self-healing pipeline (Kavach replaces N8N). `capture` gathers CI/bug-hunt
    /// failure context and writes a self-heal roadmap card the loop will dispatch.
    #[command(
        after_help = "EXAMPLES:\n  kavach heal capture --project P --incident run-42 --summary 'smoke test failed' --log ci.log\n\nWHEN: a CI run failed or a bug-hunt found a defect — enqueue it for the autonomous loop to fix. Kavach never calls an LLM; the subscription agent heals."
    )]
    Heal {
        #[command(subcommand)]
        action: HealAction,
    },
    /// Meta-Harness Loophole Loop: hunt loopholes in the SYSTEM ITSELF across the
    /// six attack lenses, record each to the Kavach DB, and capture a heal card so
    /// the loop fixes it — then re-hunt until dry. Emits a per-iteration YAML to a
    /// /tmp working dir to precisely target each round's unit of work.
    #[command(
        after_help = "EXAMPLES:\n  kavach loophole sweep --project P\n\nWHEN: continuously self-interrogate the codebase for concurrency/failure/malformed/authz/replay/boundary loopholes. Kavach detects (non-AI) + records; the subscription agent fixes."
    )]
    Loophole {
        #[command(subcommand)]
        action: LoopholeAction,
    },
    /// Print a vendor's LIVE upstream hook-contract schema source so an operator
    /// or agent can fetch + diff the current contract (realtime drift awareness).
    #[command(
        after_help = "EXAMPLES:\n  kavach schema --all\n  kavach schema --vendor cursor\n\nWHEN: a native tool may have changed its hook format — reference the live schema URL instead of a frozen assumption."
    )]
    Schema {
        /// Which tool: cc | cursor | codex | antigravity (agy) | gemini (alias).
        /// Omit with `--all` to list every vendor.
        #[arg(long)]
        vendor: Option<String>,
        /// List the schema source for every vendor.
        #[arg(long)]
        all: bool,
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
    /// Self-audit kavach's OWN source for silent-failure / unproven-DELETE patterns (read-only)
    #[command(
        after_help = "Scans kavach-engine/session/rpc/surreal. Exit 1 if findings. Add `// doctor:ok` to silence a reviewed benign line."
    )]
    Doctor,
    /// Manage SDLC development phases (PLAN/IMPLEMENT/TEST/HARDEN)
    #[command(after_help = "See: kavach phase --help")]
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
    #[command(after_help = "See: kavach loop --help")]
    Loop {
        #[command(subcommand)]
        action: LoopAction,
    },
    /// Verify a roadmap entry: cargo check + tests, then done→verified.
    #[command(
        long_about = "Writer-is-not-evaluator separation: runs `cargo check` and \
crate tests for the unit, then transitions the roadmap row from done to verified \
only on pass. SOURCE: 42-pattern catalog §3.5.",
        after_help = "EXAMPLES:\n  kavach verify --project nicole-carpenter --key roadmap.unit.foo --crate-name my-crate\n\nWHEN: After implementation is done and `db status-update … --status done`."
    )]
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
        /// Record done→verified without running cargo (deploy proven live). Requires --proof.
        #[arg(long)]
        external_verified: bool,
        /// Audit proof (URL/sha/receipt) persisted on the card; required with --external-verified.
        #[arg(long)]
        proof: Option<String>,
    },
    /// One-shot deploy: build, test, install binary to ~/.local/bin/kavach.
    #[command(
        after_help = "EXAMPLES:\n  just install              # from kavach-rs (plain CLI)\n  kavach deploy --skip-tests\n\nWHEN: After engine/cli changes; restarts RPC daemon on success."
    )]
    Deploy {
        /// Skip the cargo nextest step (build + install only).
        #[arg(long)]
        skip_tests: bool,
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
    #[command(
        after_help = "EXAMPLES:\n  kavach pipeline plan --project P --spec-key app_spec.build\n  kavach pipeline status --project nicole-carpenter\n\nWHEN: Bootstrap roadmap from app_spec; track multi-stage workflow counts."
    )]
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
    /// Unified JSON snapshot: kanban counts, session, phase, loop state.
    #[command(
        long_about = "Emits one JSON object for agent harness startup: project slug, \
session id/phase, loop active flag, kanban status histogram, and top roadmap rows.\n\n\
Uses direct SurrealDB (same path as `db kanban`) — reliable when RPC socket is sandboxed.",
        after_help = "EXAMPLES:\n  kavach context --project nicole-carpenter --limit 10\n  kavach context --project P --status in_progress --key backend\n\nWHEN: Prefer over piping stop hooks when you only need backlog visibility."
    )]
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
    /// Think-mode: hybrid keyword+graph retrieval over the kavach memory corpus.
    /// Emits cited, RRF-ranked hits as JSON; auto-files a research card when the
    /// corpus is thin for the query (gap-analysis closes the self-improving loop).
    /// SOURCE: roadmap.unit.harness.brain-os.g2-think-mode.
    #[command(
        after_help = "EXAMPLES:\n  kavach think --project kavach-rs \"hybrid retrieval design\"\n  kavach think --project P --limit 5 \"lease fencing epoch\"\n\nWHEN: recover prior decisions/research before re-researching from scratch."
    )]
    Think {
        /// Project slug (scopes the gap-file write).
        #[arg(long)]
        project: String,
        /// Free-text retrieval query (BM25 over title/content + concept FTS).
        query: String,
        /// Max fused hits to return (default: 10).
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    /// Provision the Rust CLI toolbelt the gates enforce (rg, fd, bat, eza, …)
    /// via `cargo binstall` — ships *with* kavach, no per-machine setup.
    /// SOURCE: arch.decision.toolbelt-binstall-subcommand.
    Toolbelt {
        #[command(subcommand)]
        action: ToolbeltAction,
    },
    /// Install the per-language strict-rules profile (Rust/TS/Go) so the build
    /// FAILS on bad patterns — language-agnostic, no suppression.
    /// SOURCE: decision.lint.language-profile-template.
    Lint {
        #[command(subcommand)]
        action: LintAction,
    },
    /// Print the FULL command tree (every command → subcommand → leaf) or a complete Markdown reference.
    #[command(
        name = "commands",
        after_help = "EXAMPLES:\n  kavach commands                 # indented tree of every path + summary\n  kavach commands --tree          # same (explicit)\n  kavach commands --markdown      # full reference: flags, defaults, examples\n  kavach commands --markdown > docs/CLI.md\n\nWHEN: discover the whole surface at once, or (re)generate docs/CLI.md. Walks the live clap tree — never drifts."
    )]
    CommandTree {
        /// Render the indented command tree (default when neither flag is given).
        #[arg(long)]
        tree: bool,
        /// Render the complete Markdown reference (commands, flags, defaults, examples).
        #[arg(long)]
        markdown: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum ServersAction {
    /// Ensure the `SurrealDB` store + web UI are running; print the URL.
    Up {
        /// Web UI port (loopback). Default 7777 (unprivileged).
        #[arg(long, default_value_t = kavach_web::DEFAULT_PORT)]
        port: u16,
    },
    /// Stop the web UI (the launchd-owned `SurrealDB` server is left running).
    Down {
        /// Web UI port to stop. Default 7777.
        #[arg(long, default_value_t = kavach_web::DEFAULT_PORT)]
        port: u16,
    },
    /// Report up/down for the `SurrealDB` store + web UI.
    Status {
        /// Web UI port to probe. Default 7777.
        #[arg(long, default_value_t = kavach_web::DEFAULT_PORT)]
        port: u16,
    },
}

#[derive(Subcommand)]
pub(crate) enum HealAction {
    /// Gather a failure's context (logs, changed files) and write its self-heal
    /// roadmap card. Idempotent on `--incident` (re-capture updates one card).
    Capture {
        /// Project slug the card belongs to.
        #[arg(long)]
        project: String,
        /// Stable incident id (CI run id / bug-hunt finding id) — the card key.
        #[arg(long)]
        incident: String,
        /// One-line failure summary (the card title).
        #[arg(long)]
        summary: String,
        /// Path to the build/test log to tail into the card (optional).
        #[arg(long)]
        log: Option<String>,
        /// Git ref to diff against for changed files (default: HEAD~1).
        #[arg(long, default_value = "HEAD~1")]
        diff_base: String,
    },
    /// Proactive bug-hunt: run the repo's non-AI quality gates (cargo check,
    /// clippy -D warnings, machete) and, for each FAILING gate, capture a
    /// self-heal card so the loop fixes the defect BEFORE CI does. Idempotent
    /// per gate (re-sweep updates one card per gate). Kavach never calls an LLM.
    Sweep {
        /// Project slug the cards belong to.
        #[arg(long)]
        project: String,
    },
    /// Fail-closed auto-merge decision for a heal PR. ALLOWS merge ONLY when all
    /// hold: the master switch is ON (env `KAVACH_HEAL_AUTOMERGE=1`, default OFF)
    /// AND CI is green AND 3-witness passed AND the diff touches NO protected
    /// path. Exit 0 = allow, non-zero = deny (and prints every failing reason).
    /// Kavach decides; it does not itself perform the merge.
    MergeGate {
        /// PR number to evaluate (drives `gh pr checks` + `gh pr diff`).
        #[arg(long)]
        pr: u64,
        /// Assert the heal contract's 3-witness verification passed for this PR.
        /// Absent → treated as NOT verified (fail-closed).
        #[arg(long)]
        witness_pass: bool,
    },
    /// Ingestion bridge (host-side): poll OPEN GitHub Issues labelled `self-heal`
    /// (opened by the CI self-heal workflow), capture each as a local roadmap
    /// card via the RPC single-writer path, then relabel the issue
    /// `self-heal-queued` so it is ingested exactly once. Connects the runner
    /// issue queue (H2) to the local card (H1). Kavach never calls an LLM.
    Ingest {
        /// Project slug the captured cards belong to.
        #[arg(long)]
        project: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum LoopholeAction {
    /// Run ONE loophole-hunt round: emit the iteration YAML to /tmp, scan the
    /// six attack lenses over the workspace, record each finding to the DB
    /// (mistakes) and capture a heal card. Idempotent per (lens, site).
    Sweep {
        /// Project slug the findings/cards belong to.
        #[arg(long)]
        project: String,
        /// Sweep run id (groups iterations; defaults to a fixed `adhoc` id so a
        /// re-run is idempotent on the same findings). Override per scheduled run.
        #[arg(long, default_value = "adhoc")]
        run_id: String,
        /// 1-based round number within the run (the loop-until-dry counter).
        #[arg(long, default_value_t = 1)]
        round: u32,
    },
    /// Loop-until-dry: re-run sweep rounds (each emits its own /tmp iteration
    /// YAML) until `dry_rounds` consecutive rounds surface NO new (lens, site)
    /// finding, OR `max_rounds` is hit. This is the meta-harness loop that keeps
    /// hunting + recording until the system converges. Bounded; never infinite.
    Loop {
        /// Project slug the findings/cards belong to.
        #[arg(long)]
        project: String,
        /// Run id grouping every round's /tmp iteration YAML.
        #[arg(long, default_value = "loop")]
        run_id: String,
        /// Consecutive no-new-finding rounds that declare convergence (dry).
        #[arg(long, default_value_t = 2)]
        dry_rounds: u32,
        /// Hard cap on rounds — the runaway brake (never loops forever).
        #[arg(long, default_value_t = 10)]
        max_rounds: u32,
    },
    /// Install the PROACTIVE host schedule: a code-owned launchd `LaunchAgent`
    /// that runs `kavach loophole loop` on a daily calendar interval. The third
    /// trigger (on-demand CLI + stop-gate hook + this cron) of the meta-harness loop.
    Cron {
        /// Project slug the scheduled loop hunts + records findings for.
        #[arg(long)]
        project: String,
        /// Hour of day (0–23, local time) the daily loop fires.
        #[arg(long, default_value_t = 4)]
        hour: u8,
        /// Render the plist to stdout instead of writing it (no filesystem change).
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum ToolbeltAction {
    /// Fetch + install the toolbelt's prebuilt binaries into the cargo bin dir.
    Install {
        /// Pass `--no-confirm` to `cargo binstall` (non-interactive).
        #[arg(long)]
        yes: bool,
        /// Restrict to a comma-separated subset of bin names (e.g. `rg,fd,bat`).
        #[arg(long)]
        only: Option<String>,
    },
    /// List every toolbelt tool with its provider crate + upstream license.
    List,
}

#[derive(Subcommand)]
pub(crate) enum LintAction {
    /// Detect the stack(s) here and install the strict-rules manifest for each.
    Init {
        /// Project root to operate on (default: walk up from cwd to a manifest).
        #[arg(long)]
        path: Option<String>,
        /// Report what would be written without touching any file.
        #[arg(long)]
        dry_run: bool,
    },
    /// Whole-repo over-engineering scan: ranked delete/stdlib/native/yagni/shrink.
    Audit {
        /// Project root to scan (default: walk up from cwd to a manifest).
        #[arg(long)]
        path: Option<String>,
    },
    /// Harvest simplification-ceiling markers into a debt ledger.
    Debt {
        /// Project root to scan (default: walk up from cwd to a manifest).
        #[arg(long)]
        path: Option<String>,
    },
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

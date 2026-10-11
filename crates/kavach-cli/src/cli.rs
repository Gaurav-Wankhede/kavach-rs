// hub: clap CLI entry point — Cli struct and Commands enum are intentionally here
// ARCH: see kavach db get --category decision --key arch.decision.fourteen_prefix_const_table
use clap::{Parser, Subcommand};

pub(crate) mod db;
mod harness_loop;
pub(crate) mod help_md;
mod help_stack;
pub(crate) mod help_tree;
mod oversized;

mod rules;
mod security;
mod session;
mod todos;
mod working_state;

pub(crate) use db::DbAction;
pub(crate) use harness_loop::LoopAction;
pub(crate) use oversized::OversizedFormat;

pub(crate) use rules::RulesAction;
pub(crate) use security::SecurityAction;
pub(crate) use session::SessionAction;

pub(crate) use working_state::WorkingStateAction;
// PipelineAction is defined inline in this file (initializer→subagent pipeline).

/// Build version string emitted by build.rs as `<iso>+git:<sha>`.
/// Read at compile time so `kavach --version` returns the exact build identity.
pub(crate) const KAVACH_VERSION: &str = env!("KAVACH_VERSION");
pub(crate) const KAVACH_BUILD_TIMESTAMP: &str = env!("KAVACH_BUILD_TIMESTAMP");
pub(crate) const KAVACH_GIT_SHA: &str = env!("KAVACH_GIT_SHA");

#[derive(Parser)]
#[command(
    name = "kavach",
    about = "Kavach harness — hooks, embedded Turso kanban, SDLC phases, agent loop control",
    long_about = "Kavach wraps AI agent sessions with enforcement hooks, an embedded Turso \
memory store (projects, roadmap/kanban, decisions, graph), and SDLC phase gates.\n\n\
WORKFLOWS (common agent paths):\n  \
Session start:  kavach db kanban --project <slug>\n  \
                kavach db get --project <slug> --category roadmap --key <unit> --full\n  \
                kavach context --project <slug>\n  \
Close a card:   kavach db status-update … --status done\n  \
                kavach verify --project <slug> --key <unit> --crate-name <crate>\n  \
                kavach db kanban-close --project <slug> --key <unit>\n  \
Health check:   kavach db kanban && kavach context (direct DB — no RPC required)\n\n\
ZERO-GREP LOOKUPS (use these BEFORE rg/grep — save tokens):\n  \
Where declared: kavach origin <SYMBOL> [<SYMBOL>...]   # var/fn/param/type/enum-variant/const -> file:line\n  \
Find bugs:      kavach hunt [PATH]                      # regex worst-practice sweep, no LLM\n\n\
Run `kavach <command> --help` for subcommands, flags, and examples.",
    version = KAVACH_VERSION
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Fast, gitignore-aware path and file search (zero-LLM, replaces fd/find/origin).
    Glob {
        /// Glob pattern to match against paths (e.g. "*.rs", "**/*.ts")
        pattern: String,
        /// Root path to search (default: workspace root / current dir)
        #[arg(default_value = ".")]
        path: String,
        /// Filter by entry type: f (file) or d (directory)
        #[arg(short = 't', long = "type")]
        entry_type: Option<char>,
        /// Search hidden files
        #[arg(long)]
        hidden: bool,
        /// Maximum directory depth
        #[arg(long)]
        max_depth: Option<usize>,
        /// Max results to emit (default: 100)
        #[arg(short = 'm', long = "max-count", default_value_t = 100)]
        max_count: usize,
    },

    /// High-precision polyglot AST and structural code search (replaces sg/rg).
    Grep {
        /// AST or text pattern to match (optional when --def, --call, or --type is supplied)
        #[arg(default_value = "")]
        pattern: String,
        /// Path to search (file or directory, defaults to current dir)
        #[arg(default_value = ".")]
        path: String,
        /// Treat pattern as an AST-grep pattern (default: true)
        #[arg(short = 'p', long = "ast", default_value_t = true)]
        ast: bool,
        /// Target language (rust, python, typescript, javascript, go, c, cpp, java, bash, etc.)
        #[arg(short = 'l', long = "lang")]
        lang: Option<String>,
        /// File glob filter (e.g. "*.rs")
        #[arg(short = 'g', long = "glob")]
        glob: Option<String>,
        /// Emit compact JSON records instead of file:line text
        #[arg(long)]
        json: bool,
        /// Max matches to emit (default: 50)
        #[arg(short = 'm', long = "max-count", default_value_t = 50)]
        max_count: usize,
        /// Show NUM lines of trailing and leading context (retains exact line spacing)
        /// Case-insensitive search
        #[arg(short = 'i', long = "ignore-case")]
        ignore_case: bool,
        /// Match whole words only
        #[arg(short = 'w', long = "word-regexp")]
        word_regexp: bool,
        /// Extract full enclosing AST semantic scope (fn, struct, class) instead of raw line numbers
        #[arg(short = 's', long = "scope")]
        scope: bool,
        /// Match definitions/declarations of a symbol across 28 AST languages
        #[arg(long = "def")]
        def: Option<String>,
        /// Match invocation/call sites of a symbol across 28 AST languages
        #[arg(long = "call")]
        call: Option<String>,
        /// Match type definitions or type annotations across 28 AST languages
        #[arg(long = "type")]
        type_name: Option<String>,

    },

    /// Surgical in-place file text/regex replacement (replaces sd).
    Replace {
        /// String or regex pattern to search for
        find: String,
        /// Replacement content
        replace: String,
        /// Target file path
        path: String,
        /// Treat pattern as literal fixed string (default: true)
        #[arg(short = 'F', long = "fixed-strings", default_value_t = true)]
        fixed_strings: bool,
        /// Treat pattern as regular expression
        #[arg(short = 'r', long = "regex")]
        regex: bool,
        /// Show diff without modifying file
        #[arg(long)]
        dry_run: bool,
    },

    /// Trace symbol definition, callers, and dependency context network (aliases: graph, network).
    Trace {
        /// Symbol identifier to trace (function, struct, type, const)
        symbol: String,
        /// Target directory or file to analyze (default: current directory)
        #[arg(default_value = ".")]
        path: String,
        /// Maximum hop depth for relationships
        #[arg(short = 'd', long = "depth", default_value_t = 1)]
        depth: usize,
        /// Show only inbound callers/references
        #[arg(long)]
        callers: bool,
        /// Show only outbound callees/dependencies
        #[arg(long)]
        callees: bool,
        /// Emit structured JSON output
        #[arg(long)]
        json: bool,
        /// Emit Mermaid graph/flowchart format
        #[arg(long)]
        mermaid: bool,
    },

    /// Show session status, build identity, and enforcement flags.
    #[command(
        after_help = "EXAMPLES:\n  kavach status\n\nWHEN: session-start sanity check; confirms project slug and pending gates."
    )]
    Status,
    /// Start/stop/inspect the background servers (Turso DB store + web UI).
    #[command(
        after_help = "EXAMPLES:\n  kavach servers up        # ensure DB + GUI running, print URL\n  kavach servers status\n  kavach servers down\n\nWHEN: bring the GUI online, or check why pages show the offline panel."
    )]
    /// Query and project JSON state with zero-bloat TOON tabular formatting
    Json {
        /// Dot-path query projection (e.g. .rust_toolbelt.mapping[0].tool)
        #[arg(default_value = ".")]
        query: String,
        /// Target JSON file path (or stdin)
        path: Option<String>,
        /// Emit newline-separated keys without quotes
        #[arg(short = 'k', long = "keys")]
        keys: bool,
        /// Emit unquoted raw string scalar
        #[arg(short = 'r', long = "raw")]
        raw: bool,
        /// Emit compact standard JSON instead of TOON tabular format
        #[arg(short = 'j', long = "json")]
        json: bool,

    },

    /// High-precision YAML projection to clean TOON output with atomic in-place mutation
    Yaml {
        /// Dot-path query projection (e.g. .services.db.image)
        #[arg(default_value = ".")]
        query: String,
        /// Target YAML file (omit to read from stdin)
        path: Option<String>,
        /// Emit bare keys for targeted mapping
        #[arg(short = 'k', long)]
        keys: bool,
        /// Emit raw unquoted scalar strings
        #[arg(short = 'r', long)]
        raw: bool,
        /// Emit JSON representation instead of clean TOON
        #[arg(long)]
        json: bool,

    },

    /// High-precision TOML projection to clean TOON output with atomic in-place mutation
    Toml {
        /// Dot-path query projection (e.g. .package.version, .dependencies)
        #[arg(default_value = ".")]
        query: String,
        /// Target TOML file (omit to read from stdin)
        path: Option<String>,
        /// Emit bare keys for targeted table
        #[arg(short = 'k', long)]
        keys: bool,
        /// Emit raw unquoted scalar strings
        #[arg(short = 'r', long)]
        raw: bool,
        /// Emit JSON representation instead of clean TOON
        #[arg(long)]
        json: bool,

    },

    /// Zero-secret leakage environment variable manager for safe LLM inspection & refactoring
    Env {
        /// Target .env file (default: .env)
        path: Option<String>,
        /// Emit bare variable keys only (zero secret tokens)
        #[arg(short = 'k', long)]
        keys: bool,
        /// Safely rename a key in-place preserving its secret value byte-for-byte (format: OLD:NEW)
        #[arg(long)]
        rename: Option<String>,
        /// Atomically remove a variable key and value from .env
        #[arg(long)]
        unset: Option<String>,
        /// Audit code lookups (std::env::var, process.env) against .env without reading secret values
        #[arg(long)]
        diff: Option<String>,
    },

    /// High-precision HTML DOM extraction using CSS selectors (powered by scraper)
    Html {
        /// CSS Selector to match (e.g. "h1", "div.card > a", "meta[property='og:title']")
        selector: String,
        /// Target HTML file path (or stdin)
        path: Option<String>,
        /// Extract specific attribute value instead of inner HTML (e.g. href, src, content)
        #[arg(long)]
        attr: Option<String>,
        /// Strip inner HTML tags and emit plain text only
        #[arg(short = 't', long)]
        text: bool,
        /// Output count of matched elements only
        #[arg(short = 'c', long)]
        count: bool,
    },

    /// Specialized 2D vector graphic DOM inspection, color mutation, and viewBox scaling
    Svg {
        /// Target SVG file path
        path: String,
        /// Extract path definition coordinates (d="...") only
        #[arg(long)]
        paths: bool,

    },
    Servers {
        #[command(subcommand)]
        action: ServersAction,
    },
    /// Run a gate hook (called by Claude Code / Cursor hooks).
    #[command(
        after_help = "EXAMPLES:\n  kavach gates stop --help          # gate purpose (no stdin)\n  echo '{\"hook_event_name\":\"Stop\",\"cwd\":\".\"}' | kavach gates stop --hook --vendor cursor\n\nWHEN: IDE hooks only. For kanban health use `kavach db kanban` or `kavach context`."
    )]
    Gates {
        /// Gate name (pre-write, post-write, pre-tool, post-tool, intent, stop, session-start, …)
        gate_name: String,
        /// Read JSON input from stdin (hook mode)
        #[arg(long)]
        hook: bool,
        /// Verify a prompt against a gate without hook mode (dry-run)
        #[arg(long)]
        verify: Option<String>,
        /// Force the harness dialect (claude-code|cursor|codex|antigravity|pi|kimi).
        /// Omit to auto-detect from the payload (falls back to `$KAVACH_HARNESS`,
        /// then Claude Code). Lets one hook command serve every harness.
        #[arg(long)]
        vendor: Option<String>,
    },
    /// Manage session lifecycle
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
    /// Manage the embedded Turso memory store (projects, kanban, decisions, graph).
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
    #[command(hide = true)]
    Heal {
        #[command(subcommand)]
        action: HealAction,
    },
    #[command(hide = true)]
    Loophole {
        #[command(subcommand)]
        action: LoopholeAction,
    },
    /// Unified zero-LLM code auditor: one command over all four lenses (yagni · self · worst-practice · security)
    #[command(
        after_help = "Consolidates `lint audit` + `doctor` + `hunt` + `loophole` into ONE walker / Finding / command. Exit 1 if findings. No LLM."
    )]
    Audit {
        /// Directory to scan (default: workspace root / current dir)
        path: Option<std::path::PathBuf>,
        /// Which lens(es) to run: all | code | self | security
        #[arg(long, default_value = "all")]
        lens: String,
        /// Also run the rustc + clippy backend (slower; requires a compilable cargo project)
        #[arg(long)]
        deep: bool,
        /// Write self-heal cards for security-lens findings (opt-in; default report-only)
        #[arg(long)]
        fix_cards: bool,
    },

    /// Find a symbol's ACTUAL declaration / centralized-config origin (zero-LLM, terse file:line)
    #[command(
        after_help = "Resolves where NAME(s) are declared (env-var/const/config-field/fn/type/let/param/enum-variant), config-origins first.\n\nEXAMPLES:\n  kavach origin DATABASE_URL\n  kavach origin DATABASE_URL retry_limit\n  kavach origin DATABASE_URL --all\n  kavach origin --query '{\"value_regex\":\"^postgres://\",\"consumed_by\":[\"PgPool\"],\"env_key_hints\":[\"db\"],\"name_aliases\":[\"DATABASE_URL\"]}'\n\nUSE THIS INSTEAD OF grep/rg to find where a symbol lives — zero LLM tokens. Role-query fields are all optional; a value_regex alone resolves an arbitrarily-named var."
    )]
    Origin {
        /// Symbol name(s) to resolve (var, const, fn, type, env-var, config field, param, enum-variant)
        names: Vec<String>,
        /// Directory to search (default: workspace root / current dir)
        #[arg(long)]
        path: Option<std::path::PathBuf>,
        /// Role-query JSON (resolve by ROLE not NAME); schema in EXAMPLES below
        #[arg(long)]
        query: Option<String>,
        /// Read the role-query JSON from stdin instead of --query
        #[arg(long, conflicts_with = "query")]
        stdin: bool,
        /// Show every site, not just the top one + count
        #[arg(long)]
        all: bool,
    },
    #[command(hide = true)]
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
    #[command(hide = true)]
    Pipeline {
        #[command(subcommand)]
        action: PipelineAction,
    },
    #[command(hide = true)]
    Security {
        #[command(subcommand)]
        action: SecurityAction,
    },

    #[command(hide = true)]
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
        /// Return only changed fields since last call in this session
        #[arg(long)]
        delta: bool,
        /// Emit full raw JSON payload instead of default 1-line badge
        #[arg(long)]
        full: bool,
    },
    /// Think-mode: hybrid keyword+graph retrieval over the kavach memory corpus
    Think {
        /// Project slug (scopes the gap-file write).
        #[arg(long)]
        project: String,
        /// Free-text retrieval query (BM25 over title/content + concept FTS).
        query: String,
        /// Max fused hits to return (default: 10).
        #[arg(long, default_value_t = 10)]
        limit: usize,
        /// Weight scoring by session recency and recall frequency
        #[arg(long)]
        session_weighted: bool,
    },


    #[command(hide = true)]
    Lint {
        #[command(subcommand)]
        action: LintAction,
    },
    /// Retrieve full uncompressed payload from disk CCR cache by hash.
    #[command(
        after_help = "EXAMPLES:\n  kavach retrieve <hash>\n\nWHEN: inspect full tool output or payload that was cached to save LLM context budget."
    )]
    #[command(hide = true)]
    Retrieve {
        /// Hash or prefix of the cached CCR blob.
        hash: String,
    },


    #[command(hide = true)]
    WorkingState {
        #[command(subcommand)]
        action: WorkingStateAction,
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

    /// Sovereign, zero-bloat in-process Git inspection suite (log, show, diff, status)
    Git {
        #[command(subcommand)]
        action: GitAction,
    },
}

#[derive(Subcommand)]
pub(crate) enum ServersAction {
    /// Ensure the Turso DB store + web UI are running; print the URL.
    Up {
        /// Web UI port (loopback). Default 7777 (unprivileged).
        #[arg(long, default_value_t = kavach_web::DEFAULT_PORT)]
        port: u16,
    },
    /// Stop the web UI (the launchd-owned Turso store is embedded — no separate server).
    Down {
        /// Web UI port to stop. Default 7777.
        #[arg(long, default_value_t = kavach_web::DEFAULT_PORT)]
        port: u16,
    },
    /// Report up/down for the Turso DB store + web UI.
    Status {
        /// Web UI port to probe. Default 7777.
        #[arg(long, default_value_t = kavach_web::DEFAULT_PORT)]
        port: u16,
    },
    /// Launch the HTMX dashboard on <http://127.0.0.1>:<port>.
    #[command(
        after_help = "EXAMPLES:\n  kavach servers ui              # serve on :7777\n  kavach servers ui --port 8080\n\nWHEN: browse projects/roadmap/kanban/decisions/knowledge in a browser."
    )]
    Ui {
        /// TCP port to bind on loopback (default 7777, unprivileged).
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
    /// Scan a directory for files whose code-LOC exceeds the threshold.
    Scan {
        /// Directory to scan (default: workspace root / current dir).
        #[arg(long, default_value = ".")]
        dir: String,
        /// Code-LOC threshold (files above this are flagged).
        #[arg(long, default_value_t = 500)]
        threshold: u32,
        /// Output format: text or json.
        #[arg(long, value_enum, default_value_t = OversizedFormat::Text)]
        format: OversizedFormat,
    },
    /// Skill registry: list, check, generate, show rules.
    Rules {
        #[command(subcommand)]
        action: RulesAction,
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

#[derive(Subcommand)]
pub(crate) enum GitAction {
    /// Zero-bloat commit log in clean TOON format
    Log {
        /// Number of commits to show (default: 5)
        #[arg(short = 'n', default_value_t = 5)]
        limit: usize,
        /// Format as compact single-line hashes and subjects
        #[arg(long)]
        oneline: bool,
    },
    /// Surgical revision object retrieval with AST semantic scoping (zero whole-file spam)
    Show {
        /// Target revision and path (<rev>:<path>, e.g. HEAD:src/main.rs)
        target: String,
        /// Extract definition of function, struct, or enum
        #[arg(long)]
        def: Option<String>,
        /// Find callers of symbol in historical file
        #[arg(long)]
        call: Option<String>,
    },
    /// Working tree diff telemetry and status without raw unbounded diff dumps
    Diff {
        /// Output summary line only (files changed, insertions, deletions)
        #[arg(long)]
        stat: bool,
        /// List modified file paths only
        #[arg(long)]
        files: bool,
        /// Extract modified symbols (functions, structs, enums) using AST
        #[arg(long)]
        symbols: bool,
    },
    /// In-process working tree clean/modified status
    Status,
    /// In-process branch listing and current branch telemetry
    Branch,
    /// In-process commit of staged or modified tracked files
    Commit {
        /// Commit message
        #[arg(short, long)]
        message: String,
        /// Automatically stage all modified tracked files
        #[arg(short, long)]
        all: bool,
    },
}

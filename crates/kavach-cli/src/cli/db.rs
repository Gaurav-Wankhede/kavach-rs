//! Single clap `Subcommand` enum — the `db` command surface.
//! One enum = one cohesive CLI contract; splitting variants across
//! files fragments the command definition with no reuse gain (clap derive
//! needs them in one enum). Variant handlers live in `cmd/db/*` (already split).
use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum DbAction {
    /// Register a project with absolute path
    #[command(
        after_help = "EXAMPLES:\n  kavach db register --slug my-app --path /abs/path --stack 'rust|axum'\n\nWHEN: First-time onboarding of a repo so kavach can bind sessions to it."
    )]
    Register {
        /// Project slug (unique identifier)
        #[arg(long)]
        slug: String,
        /// Absolute path to project root
        #[arg(long)]
        path: String,
        /// Tech stack (e.g. "rust|axum")
        #[arg(long)]
        stack: Option<String>,
    },
    /// Query memory entries for a project
    #[command(
        after_help = "EXAMPLES:\n  kavach db query --project P --category roadmap\n  kavach db query --project P --category decision --depth 400 --all\n\nWHEN: Browse a category's rows; add --depth for body text, --all to include done items."
    )]
    Query {
        /// Project slug
        #[arg(long)]
        project: String,
        #[arg(long, help = crate::cmd::db::write::CATEGORY_HELP)]
        category: Option<String>,
        /// Include DONE items (filtered out by default for roadmap)
        #[arg(long)]
        all: bool,
        /// Per-row content depth: a char count (e.g. `--depth 400`) or `all` for
        /// the whole body. Omitted prints titles only (breadth). `KAVACH_NO_TRUNCATE=1`
        /// forces `all` everywhere.
        #[arg(long)]
        depth: Option<String>,
        /// Return only the specified field per row (e.g. key, title, status)
        #[arg(long)]
        field: Option<String>,
        /// Compress content using native compression engine to save LLM context budget
        #[arg(long)]
        compact: bool,
    },
    /// Search with metadata filters (`entry_status`, since, contains)
    /// SOURCE: <https://docs.rs/clap/latest/clap>/_derive/_tutorial/index.html
    #[command(
        after_help = "EXAMPLES:\n  kavach db search --project P --category roadmap --status in_progress\n  kavach db search --project P --since 7d --contains scylla --limit 50\n\nWHEN: Narrow a category by status/recency/substring instead of listing everything."
    )]
    Search {
        /// Project slug
        #[arg(long)]
        project: String,
        #[arg(long, help = crate::cmd::db::write::CATEGORY_HELP)]
        category: Option<String>,
        /// Filter by `entry_status` (planned, todo, `in_progress`, done, verified)
        #[arg(long)]
        status: Option<String>,
        /// Filter entries updated within duration (e.g. 7d, 30d, 1h)
        #[arg(long)]
        since: Option<String>,
        /// Filter by title/content containing substring
        #[arg(long)]
        contains: Option<String>,
        /// Max results (default: 20)
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Write a memory entry. Strict mode (`roadmap/decision/research/pattern/app_spec)`:
    /// pass either `--new` (create — fuzzy-checks against existing titles) or
    /// `--update-key <existing>` (update a known row). Plain `--key` without
    /// either flag is rejected to prevent stale duplicate rows.
    /// SOURCE: <https://docs.rs/clap/latest/clap>/_derive/_tutorial/index.html
    #[command(
        after_help = "EXAMPLES:\n  kavach db write --project P --category roadmap --key K --title T --new\n  kavach db write --project P --category decision --key K --title T --update-key K < body.md\n\nWHEN: Persist a decision/roadmap/research row. Always pass --new (create) or --update-key (edit)."
    )]
    Write {
        /// Project slug
        #[arg(long)]
        project: String,
        #[arg(long, help = crate::cmd::db::write::CATEGORY_HELP)]
        category: String,
        /// Entry key (unique within project+category)
        #[arg(long)]
        key: String,
        /// Entry title
        #[arg(long)]
        title: String,
        /// Content. If omitted, the body is read from a piped/redirected stdin
        /// (e.g. `... < plan.md` or `cat plan.md | ...`); an interactive
        /// terminal with no pipe stores the title only.
        #[arg(long)]
        content: Option<String>,
        /// Intent: create a brand-new row. Gate fuzzy-matches title against
        /// existing rows in same category; refuses if similarity >= 0.85.
        /// Mutually exclusive with --update-key.
        #[arg(long, conflicts_with = "update_key")]
        new: bool,
        /// Intent: update an existing row by its key. CLI verifies the key
        /// exists in same project+category; refuses if not found.
        /// Mutually exclusive with --new.
        #[arg(long = "update-key")]
        update_key: Option<String>,
        /// Dispatch priority (roadmap/decision only). Lower number = higher
        /// rank — `--priority 1` is picked before `--priority 2`, like
        /// nice(1). Omit to leave existing value unchanged on update or
        /// NONE on insert (NONE rows sort after every prioritized row,
        /// then by `created_at` ASC).
        #[arg(long)]
        priority: Option<i64>,
        /// Declarative dependency edge(s): this card `depends_on` the given
        /// key(s). Repeatable (`--depends-on a --depends-on b`). A bare key
        /// resolves to the same project+category; a `slug/cat/key` qname is
        /// used verbatim. Feeds the DAG scheduler's topological ordering so
        /// dependents wait until each target reaches done/verified — the
        /// precise complement to the NLU prose scanner and frontmatter.
        #[arg(long = "depends-on")]
        depends_on: Vec<String>,
        /// Opus-authored executor prompt (roadmap only), served verbatim by `db next-prompt`.
        #[arg(long = "exec-prompt")]
        exec_prompt: Option<String>,
    },
    /// Fetch a single memory entry by key
    #[command(
        after_help = "EXAMPLES:\n  kavach db get --project P --category roadmap --key roadmap.unit.foo --full\n\nWHEN: Before implementing — always `--full` for roadmap units."
    )]
    Get {
        /// Project slug
        #[arg(long)]
        project: String,
        #[arg(long, help = crate::cmd::db::write::CATEGORY_HELP)]
        category: String,
        /// Entry key
        #[arg(long)]
        key: String,
        /// Show all metadata (tags, ttl, source, timestamps)
        #[arg(long)]
        full: bool,
        /// Return the SHORT snippet body (single-key get returns FULL content by
        /// default — naming one exact key is a depth request)
        #[arg(long)]
        snippet: bool,
        /// Return only the specified field (e.g. content, title, status, exec_prompt)
        #[arg(long)]
        field: Option<String>,
        /// Compress content using native compression engine to save LLM context budget
        #[arg(long)]
        compact: bool,
    },
    /// Delete specific record(s) by key or category (granular, preferred over wipe-project)
    #[command(
        after_help = "EXAMPLES:\n  kavach db delete --project P --category roadmap --key K --confirm\n  kavach db delete --project P --category research --all --dry-run\n\nWHEN: Granular removal of one row or a whole category. Prefer over wipe-project."
    )]
    Delete {
        /// Project slug
        #[arg(long)]
        project: String,
        #[arg(long, help = crate::cmd::db::write::CATEGORY_HELP)]
        category: String,
        /// Specific key to delete (omit with --all for category-wide)
        #[arg(long)]
        key: Option<String>,
        /// Delete all records in category (requires --confirm)
        #[arg(long)]
        all: bool,
        /// Confirm deletion
        #[arg(long)]
        confirm: bool,
        /// Dry-run: show what would be deleted
        #[arg(long)]
        dry_run: bool,
    },
    /// Wipe ALL data for a project (DANGEROUS — use `delete` for granular removal)
    #[command(
        after_help = "EXAMPLES:\n  kavach db wipe-project --project P --dry-run\n  kavach db wipe-project --project P --confirm\n\nWHEN: DANGEROUS — destroy ALL data for a project. Prefer `delete` for anything granular."
    )]
    WipeProject {
        /// Project slug to wipe
        #[arg(long)]
        project: String,
        /// Confirm destructive operation (DANGEROUS: deletes ALL project data)
        #[arg(long)]
        confirm: bool,
        /// Dry-run: show what would be deleted without deleting
        #[arg(long)]
        dry_run: bool,
    },
    /// Show kanban board for a project (todo / `in_progress` / done)
    #[command(
        long_about = "Direct Turso DB read (no RPC). Primary health check for agents.\n\n\
Open lanes: todo, in_progress, done. Use --include-verified for terminal verified rows.",
        after_help = "EXAMPLES:\n  kavach db kanban --project nicole-carpenter --limit 10\n  kavach db kanban --project P --status in_progress --key backend --json\n\nWHEN: Session start and after every card close — prefer over stop-hook pipes."
    )]
    Kanban {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Max items to show (default: 15, 0 = unlimited)
        #[arg(long, default_value_t = 15)]
        limit: usize,
        /// Filter by status (todo, `in_progress`, done) — open lanes only;
        /// verified/planned rows are terminal/backlog and not shown here
        /// unless --include-verified is passed.
        #[arg(long)]
        status: Option<String>,
        /// Show `in_progress` items first
        #[arg(long)]
        active_first: bool,
        /// Filter by key substring (e.g. "backend" matches "backend.crate.foo")
        #[arg(long)]
        key: Option<String>,
        /// Filter by dispatch lane (exact match). Shows only cards pinned to
        /// this lane — the board lens for affinity-sharded sessions.
        #[arg(long)]
        lane: Option<String>,
        /// Also render terminal `verified` rows in a [VERIFIED] lens. Off by
        /// default — the board shows OPEN work; this surfaces closed items
        /// (e.g. to confirm a unit reached `verified`, not just `done`).
        #[arg(long)]
        include_verified: bool,
        /// Output as JSON for agent parsing
        #[arg(long)]
        json: bool,
        /// Override the always-on dependency-DAG view.
        ///
        /// The default (no flag) is the topo-tiered text DAG, where the first
        /// tier is ready now and deeper tiers unlock as prerequisites close, with
        /// READY/BLOCKED/CYCLE markers, inline depends-on, and per-card status —
        /// the agent's always-on task awareness. Passing `mermaid` emits a
        /// `flowchart TD` for human/GUI viewing, and `--json` gives the
        /// machine-parseable card list. Reads the same declared deps the
        /// scheduler dispatches on.
        #[arg(long, value_parser = ["dag", "mermaid"])]
        format: Option<String>,
    },
    /// Close a kanban roadmap item by key (marks status=verified)
    #[command(
        after_help = "EXAMPLES:\n  kavach db kanban-close --project P --key roadmap.unit.foo\n\nWHEN: Mark a roadmap card verified once `kavach verify` has passed."
    )]
    KanbanClose {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Roadmap entry key to close (e.g. phase-1-auth)
        #[arg(long)]
        key: String,
    },
    /// Update the strict status of a memory entry (`todo|in_progress|done|verified`)
    #[command(
        after_help = "EXAMPLES:\n  kavach db status-update --project P --category roadmap --key K --status in_progress\n  kavach db status-update --project P --category roadmap --key K --status done\n  kavach db status-update --project P --category roadmap --key K --status done --verify-cmd 'bun run build'\n\nWHEN: Claim card (todo→in_progress), finish work (→done), then run `kavach verify` (→verified)."
    )]
    StatusUpdate {
        /// Project slug
        #[arg(long)]
        project: String,
        #[arg(long, help = crate::cmd::db::write::CATEGORY_HELP)]
        category: String,
        /// Entry key
        #[arg(long)]
        key: String,
        /// New status: planned, todo, `in_progress`, done, verified
        #[arg(long)]
        status: String,
        /// Non-Rust witness command to run for a roadmap `done`/`verified` promotion.
        /// Overrides `KAVACH_VERIFY_CMD` for this call. Example: `bun run build`.
        #[arg(long)]
        verify_cmd: Option<String>,
    },
    /// Append an event to the log
    #[command(
        after_help = "EXAMPLES:\n  kavach db event --type file_write --payload '{\"path\":\"x.rs\"}'\n\nWHEN: Append an audit event to the log (usually hook-driven)."
    )]
    Event {
        /// Event type (e.g. "`file_write`", "`session_start`")
        #[arg(long = "type")]
        event_type: String,
        /// JSON payload
        #[arg(long)]
        payload: Option<String>,
    },
    /// Resolve a gate-config override (project-then-global), or report the miss
    #[command(
        after_help = "EXAMPLES:\n  kavach db gate-config-get --project P --gate-key dup.block\n  kavach db gate-config-get --project '*' --gate-key dup.block   # global\n\nWHEN: Resolve a gate override (project then global), or confirm the miss."
    )]
    GateConfigGet {
        /// Project slug (`*` for the global row)
        #[arg(long)]
        project: String,
        /// Gate-config key (e.g. `dup.block`, `session.autonomy_contract`)
        #[arg(long = "gate-key")]
        gate_key: String,
    },
    /// Set a gate-config override (exactly one value flag per `--kind`)
    #[command(
        after_help = "EXAMPLES:\n  kavach db gate-config-set --project P --gate-key dup.block --kind enabled --boolean true\n  kavach db gate-config-set --project '*' --gate-key dup.threshold --kind threshold --num 0.85\n\nWHEN: Override one gate's config; pass exactly one value flag matching --kind."
    )]
    GateConfigSet {
        /// Project slug (`*` for the global row)
        #[arg(long)]
        project: String,
        /// Gate-config key
        #[arg(long = "gate-key")]
        gate_key: String,
        /// Value kind: `threshold` | `pattern_list` | `enabled` | `severity` | `text`
        #[arg(long)]
        kind: String,
        /// Numeric value (for `--kind threshold`)
        #[arg(long)]
        num: Option<f64>,
        /// Boolean value (for `--kind enabled`)
        #[arg(long)]
        boolean: Option<bool>,
        /// Comma-separated list (for `--kind pattern_list`)
        #[arg(long)]
        list: Option<String>,
        /// Text value (for `--kind severity` | `--kind text`)
        #[arg(long)]
        text: Option<String>,
    },
    /// Insert-or-update a global L0 concept (cross-project knowledge graph node)
    #[command(
        after_help = "EXAMPLES:\n  kavach db concept-add --name paseto_v4 --display 'PASETO v4' --desc '...' --tags token,crypto --sources https://...\n\nWHEN: Add/update a global L0 concept node in the cross-project graph."
    )]
    ConceptAdd {
        /// Canonical concept id (`snake_case`, e.g. `paseto_v4`)
        #[arg(long)]
        name: String,
        /// Display label (e.g. "PASETO v4")
        #[arg(long)]
        display: String,
        /// Description / prose body
        #[arg(long)]
        desc: String,
        /// Comma-separated tags (e.g. token,crypto,stateless)
        #[arg(long)]
        tags: Option<String>,
        /// Comma-separated source URLs
        #[arg(long)]
        sources: Option<String>,
    },
    /// RELATE two concepts via an ontology edge
    #[command(
        after_help = "EXAMPLES:\n  kavach db concept-link --from paseto_v4 --edge alternative_to --to jwt\n\nWHEN: Relate two concepts via an ontology edge."
    )]
    ConceptLink {
        /// Source concept name
        #[arg(long)]
        from: String,
        /// Ontology edge (`is_a`, `part_of`, `prerequisite_of`, `alternative_to`,
        /// composes, mitigates, `instance_of`, subsumes)
        #[arg(long)]
        edge: String,
        /// Target concept name
        #[arg(long)]
        to: String,
    },
    /// Full-text search concepts by description (BM25)
    #[command(
        after_help = "EXAMPLES:\n  kavach db concept-search --query 'stateless token' --limit 20\n\nWHEN: Full-text (BM25) lookup of concepts by description."
    )]
    ConceptSearch {
        /// Search terms
        #[arg(long)]
        query: String,
        /// Max results
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Add or refresh a citation (official-docs context) keyed by (project, entry-key)
    #[command(
        after_help = "EXAMPLES:\n  kavach db citation-add --project P --entry-key turso --name Turso --slug records --url https://docs.turso.tech\n\nWHEN: Pin an official-docs citation to a (project, entry-key)."
    )]
    CitationAdd {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Stable entry key (`snake_case`)
        #[arg(long = "entry-key")]
        entry_key: String,
        /// Display name (e.g. `Turso`)
        #[arg(long)]
        name: String,
        /// One metadata slug (e.g. "records")
        #[arg(long)]
        slug: String,
        /// Official-docs URL (non-empty)
        #[arg(long)]
        url: String,
    },
    /// List every citation for a project (newest-updated first)
    #[command(
        after_help = "EXAMPLES:\n  kavach db citation-list --project P\n\nWHEN: List a project's citations, newest-updated first."
    )]
    CitationList {
        /// Project slug
        #[arg(long)]
        project: String,
    },
    /// Run a read-only SQL query against the store (`SELECT` only).
    /// Ad-hoc graph inspection; mutations go through typed verbs (write, mistake-purge, …).
    #[command(
        after_help = "EXAMPLES:\n  kavach db query-raw --query 'SELECT name FROM entity LIMIT 5'\n\nWHEN: Ad-hoc read-only SQL (SELECT only); mutations go through typed verbs."
    )]
    QueryRaw {
        /// The read-only SQL to execute, e.g. `SELECT name FROM entity LIMIT 5`.
        #[arg(long)]
        query: String,
    },
    /// Print the top-priority todo card's exec_prompt to stdout (pipe to an executor model)
    #[command(
        after_help = "EXAMPLES:\n  kavach db next-prompt --project P\n\nWHEN: Get the exec_prompt for the top runnable card — feed to a cheaper model."
    )]
    NextPrompt {
        /// Project slug
        #[arg(long)]
        project: String,
    },
    /// Run multiple runnable roadmap cards in parallel, respecting dependencies
    #[command(
        after_help = "EXAMPLES:\n  kavach db parallel-run --project P --max-parallel 3\n  kavach db parallel-run --project P --max-parallel 2 --verify-cmd 'cargo test'\n\nWHEN: Run multiple runnable roadmap cards in parallel respecting dependencies."
    )]
    ParallelRun {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Maximum number of cards to run concurrently
        #[arg(long, default_value_t = 3)]
        max_parallel: usize,
        /// Override verification command (e.g. 'cargo test -p kavach-db')
        #[arg(long)]
        verify_cmd: Option<String>,
    },
    /// Author/validate machine-actionable micro-tickets (MicroTicket v1)
    #[command(subcommand)]
    Mt(MtAction),
    /// Diagnose and repair store-level drift
    #[command(subcommand)]
    Doctor(DoctorAction),
    /// Backfill vector embeddings for existing memories (Phase 5)
    #[command(
        after_help = "EXAMPLES:\n  kavach db embed-backfill --batch 50\n  kavach db embed-backfill --batch 64 --json\n\nWHEN: Populate `embedding` column for pre-vector rows; idempotent and resumable."
    )]
    EmbedBackfill {
        /// Batch size per embedding pass (default 50)
        #[arg(long, default_value_t = 50)]
        batch: usize,
        /// Output JSON instead of human table
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Flush and checkpoint WAL journal pages (PRD Phase 7 Ops discipline)
    #[command(
        after_help = "EXAMPLES:\n  kavach db checkpoint\n  kavach db checkpoint --truncate\n\nWHEN: Reclaim disk space or checkpoint transactions before backup."
    )]
    Checkpoint {
        /// Truncate WAL file to zero bytes
        #[arg(long, default_value_t = false)]
        truncate: bool,
    },
    /// Backup database snapshot with integrity check to ~/.kavach/backups/
    #[command(
        after_help = "EXAMPLES:\n  kavach db backup\n  kavach db backup --keep 5\n\nWHEN: Create a point-in-time snapshot of kavach.db."
    )]
    /// Inspect, record, or clear anti-pattern mistakes in the Knowledge Graph
    Mistakes {
        #[command(subcommand)]
        action: crate::cmd::db::mistakes::MistakesAction,
    },
    Backup {
        /// Number of recent backups to keep (default: 10)
        #[arg(long, default_value_t = 10)]
        keep: usize,
    },
}

#[derive(Subcommand)]
pub(crate) enum DoctorAction {
    /// Collapse duplicate DEPENDS_ON lines into one canonical bare-key line
    #[command(
        after_help = "EXAMPLES:\n  kavach db doctor repair-deps --project P\n  kavach db doctor repair-deps --project P --apply\n\nWHEN: Two mirrors (CLI + daemon) each appended a DEPENDS_ON line, and 52 rows carry both a project-qualified and a bare spelling of the same dependency.\n\nSAFETY: dry-run by default. Dependencies are MERGED, never dropped — a row whose qualified line names MORE targets than its bare line would lose edges if the qualified line were simply deleted."
    )]
    RepairDeps {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Write the repairs (default is a dry run that prints the plan)
        #[arg(long, default_value_t = false)]
        apply: bool,
    },
}

const MT_AFTER_HELP: &str = "\
MICROTICKET v1 — a machine-actionable ticket. Every field is validated; a
malformed ticket is REJECTED, not stored.

  kavach db mt new --project P --key mt.graph.assertions-table \\
    --kind bug --title 'Hand-roll the assertions log (no CDC in embedded libsql)' \\
    --goal 'Card history is queryable by sequence without native change capture.' \\
    --modality MUST --op add \\
    --when 'a roadmap row commits' --then 'an assertions row is appended with a monotonic seq' \\
    --scope crates/kavach-db/src/microticket.rs --do-not-touch crates/kavach-cli/src/cli/db.rs \\
    --verify-cmd 'cargo test -p kavach-db microticket::the test passes and asserts seq is monotonic' \\
    --source-kind file --source-ref crates/kavach-db/src/turso.rs:177

CONTRACT (caps are enforced, fail-closed):
  title        <=90   one line, imperative verb first
  goal         <=200  ONE sentence of intent
  change.when  <=120  trigger
  change.then  <=120  observable outcome
  scope        1-5    repo-relative paths this ticket may touch
  do_not_touch 1-5    out-of-scope globs — mandatory, it is what stops scope creep
  verify       1-5    RUN::EXPECT pairs; at least one is mandatory
  source       optional typed provenance: crate|file|url|doc|test|db
  depends_on / conflicts_with / supersedes   0-5 keys each
  conflicts_with is ENFORCED at dispatch: two conflicting tickets never share a tier.

CHARACTER CAPS ARE A DESIGN CHOICE, NOT A STANDARD. There is no published
convention for agent-ticket caps; GitHub ships min_length and omits max_length.
If a ticket does not fit, SPLIT IT — never enlarge the cap.

A VALID TICKET IS NOT A GOOD TICKET. Schema validity was measured to rise
61.5%->100% while answer accuracy FELL 19.7%->11.0% under hard decoding
(arXiv:2605.26128). Review the content; `mt new` only guarantees the shape.";

#[derive(Subcommand)]
pub(crate) enum MtAction {
    /// Create or update a micro-ticket (validated, fail-closed)
    #[command(after_help = MT_AFTER_HELP)]
    New {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Stable ticket key (kebab, <=48 chars)
        #[arg(long)]
        key: String,
        /// bug|feature|chore|refactor|perf|docs|test
        #[arg(long)]
        kind: String,
        /// One line, imperative verb first (<=90 chars)
        #[arg(long)]
        title: String,
        /// ONE sentence of intent (<=200 chars)
        #[arg(long)]
        goal: String,
        /// RFC 2119 level: MUST|SHOULD|MAY
        #[arg(long)]
        modality: String,
        /// add|edit|delete|move — exactly one operation
        #[arg(long)]
        op: String,
        /// Trigger condition (<=120 chars)
        #[arg(long)]
        when: String,
        /// Observable outcome (<=120 chars)
        #[arg(long)]
        then: String,
        /// Repo-relative path this ticket may touch (repeatable, 1-5)
        #[arg(long = "scope")]
        scope: Vec<String>,
        /// Out-of-scope glob (repeatable, 1-5, mandatory)
        #[arg(long = "do-not-touch")]
        do_not_touch: Vec<String>,
        /// 'RUN::EXPECT' pair for a shell command (repeatable)
        #[arg(long = "verify-cmd")]
        verify_cmd: Vec<String>,
        /// 'RUN::EXPECT' pair for a test path (repeatable)
        #[arg(long = "verify-test")]
        verify_test: Vec<String>,
        /// 'RUN::EXPECT' pair for an assertion (repeatable)
        #[arg(long = "verify-assert")]
        verify_assert: Vec<String>,
        /// EXPECT for a manual check (repeatable; requires --manual-by)
        #[arg(long = "verify-manual")]
        verify_manual: Vec<String>,
        /// crate|file|url|doc|test|db — provenance kind
        #[arg(long = "source-kind")]
        source_kind: Option<String>,
        /// The crate name, path[:line], or URL
        #[arg(long = "source-ref")]
        source_ref: Option<String>,
        /// Why this is the right origin (<=100 chars)
        #[arg(long = "source-note")]
        source_note: Option<String>,
        /// Ticket key that must finish first (repeatable)
        #[arg(long = "depends-on")]
        depends_on: Vec<String>,
        /// Mutually-exclusive ticket key (repeatable; enforced at dispatch)
        #[arg(long = "conflicts-with")]
        conflicts_with: Vec<String>,
        /// Ticket key this one replaces (repeatable)
        #[arg(long = "supersedes")]
        supersedes: Vec<String>,
        /// Who verifies a 'manual' step
        #[arg(long = "manual-by")]
        manual_by: Option<String>,
        /// Dispatch priority (lower runs first)
        #[arg(long)]
        priority: Option<i64>,
        /// Executor prompt served by `db next-prompt`
        #[arg(long = "exec-prompt")]
        exec_prompt: Option<String>,
        /// Validate and print without writing
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    /// Revalidate stored micro-tickets; non-zero exit if any are invalid
    #[command(
        after_help = "EXAMPLES:\n  kavach db mt check --project P\n  kavach db mt check --project P --key mt.graph\n\nWHEN: A ticket may have been written before the schema tightened, or by a loose path. Re-validate rather than trust."
    )]
    Check {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Only check keys containing this substring
        #[arg(long)]
        key: Option<String>,
    },
}

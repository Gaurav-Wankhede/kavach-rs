//! kavach:micro-file-exempt single clap `Subcommand` enum — the `db` command
//! surface. One enum = one cohesive CLI contract; splitting variants across
//! files fragments the command definition with no reuse gain (clap derive
//! needs them in one enum). Variant handlers live in `cmd/db/*` (already split).
use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum DbAction {
    /// Register a project with absolute path
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
    /// Register a sub-part within a project
    RegisterPart {
        /// Parent project slug
        #[arg(long)]
        project: String,
        /// Part name (e.g. "backend", "frontend")
        #[arg(long)]
        name: String,
        /// Absolute path to part root
        #[arg(long)]
        path: String,
        /// Part type (backend, frontend, database, mobile, infra, docs, shared, other)
        #[arg(long = "type")]
        part_type: String,
    },
    /// Query memory entries for a project
    Query {
        /// Project slug
        #[arg(long)]
        project: String,
        #[arg(long, help = crate::cmd::db::write::CATEGORY_HELP)]
        category: Option<String>,
        /// Include DONE items (filtered out by default for roadmap)
        #[arg(long)]
        all: bool,
    },
    // ALGO: CLI arg parsing — clap derive macro, no DSA
    /// Search with metadata filters (`entry_status`, since, contains)
    /// SOURCE: <https://docs.rs/clap/latest/clap>/_derive/_tutorial/index.html
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
        /// Content (reads from stdin if omitted)
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
    },
    /// Set or clear the priority of an existing entry without touching title/content.
    /// Surgical rerank verb for human-in-loop focus shifts. Refuses if the row
    /// does not exist (no implicit insert). Lower number = higher rank, like nice(1).
    PrioritySet {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Category (roadmap or decision — only these carry priority)
        #[arg(long)]
        category: String,
        /// Entry key (must already exist)
        #[arg(long)]
        key: String,
        /// New priority (lower = higher rank). Mutually exclusive with --clear.
        #[arg(long, conflicts_with = "clear")]
        priority: Option<i64>,
        /// Clear the priority (back to NONE, FIFO tail). Mutually exclusive with --priority.
        #[arg(long, conflicts_with = "priority")]
        clear: bool,
    },
    /// Pin a roadmap card to a dispatch LANE (or clear it back to unlaned).
    /// A session running `KAVACH_LANE=<name>` dispatches its own lane first,
    /// then the unlaned backlog, never a foreign lane. Refuses if the row is
    /// absent (no implicit insert). Roadmap only.
    LaneSet {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Entry key (must already exist)
        #[arg(long)]
        key: String,
        /// Lane name to pin the card to. Mutually exclusive with --clear.
        #[arg(long, conflicts_with = "clear")]
        lane: Option<String>,
        /// Clear the lane (back to the unlaned general backlog). Mutually
        /// exclusive with --lane.
        #[arg(long, conflicts_with = "lane")]
        clear: bool,
    },
    /// Sync session state to database
    Sync,
    /// Find project matching an absolute path
    FindProject {
        /// Absolute file or directory path
        #[arg(long)]
        path: String,
    },
    /// Find part matching an absolute path
    FindPart {
        /// Absolute file or directory path
        #[arg(long)]
        path: String,
    },
    /// List all registered projects
    ListProjects,
    /// Set (or clear) a project's parent, building the hierarchy
    SetParent {
        /// Child project slug
        #[arg(long)]
        child: String,
        /// Parent project slug; omit to detach to top-level
        #[arg(long)]
        parent: Option<String>,
    },
    /// Render the project hierarchy as an indented tree
    Tree,
    /// List parts for a project
    ListParts {
        /// Project slug
        #[arg(long)]
        project: String,
    },
    /// Archive expired memory entries
    Expire,
    /// Append an event to the log
    Event {
        /// Event type (e.g. "`file_write`", "`session_start`")
        #[arg(long = "type")]
        event_type: String,
        /// JSON payload
        #[arg(long)]
        payload: Option<String>,
    },
    /// Fetch a single memory entry by key
    #[command(after_help = "EXAMPLES:\n  kavach db get --project P --category roadmap --key roadmap.unit.foo --full\n\nWHEN: Before implementing — always `--full` for roadmap units.")]
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
    },
    /// Delete events older than N days (TIME-BASED — transitional, prefer `archive`)
    Rotate {
        /// Number of days to retain
        #[arg(long)]
        days: i64,
    },
    /// Archive events with no graph anchors to active roadmap (relevance-based, audit-preserving)
    Archive {
        /// Floor age in days — events younger than this are never archived
        #[arg(long, default_value_t = 30)]
        floor_days: i64,
        /// Dry-run: report what would be archived without modifying state
        #[arg(long)]
        dry_run: bool,
    },
    /// Show kanban board for a project (todo / `in_progress` / done)
    #[command(
        long_about = "Direct SurrealDB read (no RPC). Primary health check for agents.\n\n\
Open lanes: todo, in_progress, done. Use --include-verified for terminal verified rows.",
        after_help = "EXAMPLES:\n  kavach db kanban --project nicole-carpenter --limit 10\n  \
kavach db kanban --project P --status in_progress --key backend --json\n\n\
WHEN: Session start and after every card close — prefer over stop-hook pipes."
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
    },
    /// Close a kanban roadmap item by key (marks status=verified)
    KanbanClose {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Roadmap entry key to close (e.g. phase-1-auth)
        #[arg(long)]
        key: String,
    },
    /// Update the strict status of a memory entry (`todo|in_progress|done|verified`)
    #[command(after_help = "EXAMPLES:\n  kavach db status-update --project P --category roadmap --key K --status in_progress\n  kavach db status-update --project P --category roadmap --key K --status done\n\nWHEN: Claim card (todo→in_progress), finish work (→done), then run `kavach verify` (→verified).")]
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
        /// Set the structured owner-gate flag (roadmap only). `true` = the card
        /// needs an external owner action no agent can self-supply (greenlight /
        /// prod deploy / live run / secrets); the dispatcher skips it like an
        /// unmet dependency. Replaces the retired `AGENT_BLOCKED:`/`OWNER-GATED`
        /// body keywords. Omit to leave the flag unchanged.
        #[arg(long)]
        owner_gated: Option<bool>,
    },
    /// Populate the knowledge graph from existing relational data
    PopulateGraph,
    /// Backfill typed inter-entry edges (`depends_on/blocks/supersedes/references/mentions`)
    /// by re-extracting frontmatter + wikilinks from every existing memory row.
    /// Safe to re-run; UPSERT-based.
    BackfillRelationships {
        /// Optional project slug filter (default: all projects)
        #[arg(long)]
        project: Option<String>,
        /// Print planned edges without writing
        #[arg(long)]
        dry_run: bool,
    },
    /// Query the knowledge graph — list entities and their edges
    GraphQuery {
        /// Filter by entity type (skill, rule, gate, project, file, memory, session, `event_type`)
        #[arg(long)]
        entity_type: Option<String>,
        /// Show edges for a specific entity name
        #[arg(long)]
        name: Option<String>,
        /// Max entities to show (default: 20)
        #[arg(long, default_value = "20")]
        limit: usize,
    },
    /// Introspect a local `PostgreSQL` database — list tables, columns, FKs
    PgIntrospect {
        /// Postgres DSN (e.g. <postgres://localhost/mydb>)
        #[arg(long)]
        dsn: String,
    },
    /// Find isolated tables (zero incoming + outgoing FK edges)
    PgIsolation {
        /// Postgres DSN
        #[arg(long)]
        dsn: String,
    },
    /// Emit ER diagram in Mermaid format from the live schema
    PgEr {
        /// Postgres DSN
        #[arg(long)]
        dsn: String,
    },
    /// Detect likely missing FKs — columns named `<table>_id` with no declared FK
    PgDrift {
        /// Postgres DSN
        #[arg(long)]
        dsn: String,
    },
    /// Delete specific record(s) by key or category (granular, preferred over wipe-project)
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
    /// Insert-or-update a global L0 concept (cross-project knowledge graph node)
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
    ConceptSearch {
        /// Search terms
        #[arg(long)]
        query: String,
        /// Max results
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// List every concept
    ConceptList {
        /// Max results
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// Remove a single concept row by canonical name
    ConceptDelete {
        /// Concept name (`snake_case`)
        #[arg(long)]
        name: String,
    },
    /// Bulk-purge concept rows by name prefix (requires --confirm)
    ConceptDeletePrefix {
        /// Name prefix (e.g. "keyword:" to purge harvest noise)
        #[arg(long)]
        prefix: String,
        /// Required to actually run the bulk purge
        #[arg(long)]
        confirm: bool,
    },
    /// Resolve a gate-config override (project-then-global), or report the miss
    GateConfigGet {
        /// Project slug (`*` for the global row)
        #[arg(long)]
        project: String,
        /// Gate-config key (e.g. `dup.block`, `session.autonomy_contract`)
        #[arg(long = "gate-key")]
        gate_key: String,
    },
    /// Set a gate-config override (exactly one value flag per `--kind`)
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
    /// Delete a gate-config override, reverting to the compiled default
    GateConfigDelete {
        /// Project slug (`*` for the global row)
        #[arg(long)]
        project: String,
        /// Gate-config key
        #[arg(long = "gate-key")]
        gate_key: String,
    },
    /// List every gate-config override for a project (`*` for the globals)
    GateConfigList {
        /// Project slug
        #[arg(long)]
        project: String,
    },
    /// Bridge a project-scoped L1 entity to a global L0 concept
    BridgeCreate {
        /// Source table (roadmap | decision | research | pattern | `app_spec`)
        #[arg(long = "src-table")]
        src_table: String,
        /// Source `entry_key`
        #[arg(long = "src-key")]
        src_key: String,
        /// Bridge edge (implements | discusses | `references_concept` | violates)
        #[arg(long)]
        edge: String,
        /// Target concept name
        #[arg(long)]
        concept: String,
    },
    /// List every concept that any L1 entity in this project bridges to
    BridgeConceptsFor {
        /// Project slug
        #[arg(long)]
        project: String,
    },
    /// List every project (via L1 entity) that bridges to this concept
    BridgeProjectsFor {
        /// Concept name
        #[arg(long)]
        concept: String,
    },
    /// Count inbound `instance_of` edges on an `anti_pattern` (mistake recurrence)
    MistakeHitCount {
        /// Anti-pattern canonical name (e.g. `anti.self_imposed_limit.abc12345`)
        #[arg(long)]
        name: String,
    },
    /// Store an implementation-flow DAG (structured JSON ingest, render-on-read)
    FlowAdd {
        /// Project slug the flow belongs to
        #[arg(long)]
        project: String,
        /// Flow key, unique per project (e.g. `auth-flow`)
        #[arg(long)]
        key: String,
        /// Display title
        #[arg(long)]
        title: String,
        /// Path to JSON `{steps:[...],edges:[...]}` (reads stdin if omitted)
        #[arg(long = "steps-json")]
        steps_json: Option<String>,
        /// Optional raw Mermaid source cached for round-trip
        #[arg(long)]
        mermaid: Option<String>,
    },
    /// Render a stored implementation-flow DAG as Mermaid (default) or JSON
    FlowShow {
        /// Project slug
        #[arg(long)]
        project: String,
        /// Flow key
        #[arg(long)]
        key: String,
        /// Output format: `mermaid` (default) or `json`
        #[arg(long, default_value = "mermaid")]
        format: String,
    },
}

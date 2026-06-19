use std::collections::HashMap;

/// Default token budget for sonnet/haiku model class.
pub const DEFAULT_TOKEN_BUDGET: i32 = 180_000;
/// Default max output chars per subagent.
pub const DEFAULT_SUBAGENT_MAX_OUTPUT: i32 = 8_000;
/// Default total cap for all subagent output chars.
pub const DEFAULT_SUBAGENT_TOTAL_CAP: i32 = 30_000;

#[derive(Debug, Clone)]
#[expect(
    clippy::exhaustive_structs,
    reason = "SessionState is constructed at RPC handler boundary"
)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "complex state machine with many independent flags"
)]
pub struct SessionState {
    pub id: String,
    pub session_id: String,
    pub today: String,
    pub project: String,
    pub work_dir: String,
    pub training_cutoff: String,
    pub research_done: bool,
    pub research_topics: Vec<String>,
    pub memory_queried: bool,
    pub post_compact: bool,
    pub compacted_at: String,
    pub compact_count: i32,
    pub turn_count: i32,
    pub last_reinforce_turn: i32,
    pub reinforce_every_n: i32,
    pub current_task: String,
    pub task_status: String,
    pub files_modified: Vec<String>,
    pub tasks_created: i32,
    pub tasks_completed: i32,
    pub task_list_id: String,
    pub intent_type: String,
    pub intent_domain: String,
    pub intent_skills: Vec<String>,
    pub specs_injected: Vec<String>,
    pub token_budget_total: i32,
    pub token_budget_used: i32,
    pub context_phase: String,
    pub active_subagents: i32,
    pub subagent_outputs: HashMap<String, i32>,
    pub subagent_max_output: i32,
    pub subagent_total_cap: i32,
    pub team_name: String,
    pub team_members: Vec<String>,
    pub active_teammates: i32,
    pub model_id: String,
    pub modules_injected: Vec<String>,
    pub last_failure_tool: String,
    pub last_failure_turn: i32,
    /// How many times the stop gate has blocked due to this failure.
    /// Reset on `clear_failure()`. Prevents `stop_hook_active` from bypassing prematurely.
    pub failure_block_count: i32,
    /// How many times the stop gate has re-blocked because the kanban still
    /// has runnable work (the pending-work bounded breaker). Distinct from
    /// `failure_block_count`: this MUST survive a successful tool call, so
    /// `clear_failure()` deliberately does NOT touch it. Cleared only on a
    /// genuine clean stop. Without this separation, post-tool `clear_failure()`
    /// zeroed the breaker every tool call and the stop gate looped forever
    /// (perpetual "1/3", never reaching the forced-stop terminal).
    pub stop_reblock_count: i32,
    /// Per-turn gate-fire counter for the `GateBudget` circuit-breaker.
    /// Increments on every `router::emit` call; resets to 0 each new turn.
    /// SOURCE: roadmap.unit.gate-severity-router — when N≥3 gates fire on a
    /// single tool call OR M≥10 fire in a turn, the router downgrades subsequent
    /// gates to silent advise to prevent the "stack of blocks" anti-pattern.
    pub gates_fired_this_turn: i32,
    /// Per-tool-call gate counter. Resets when `last_seen_tool_use_id` changes.
    /// Pairs with `PER_CALL_BUDGET=3` in the router circuit-breaker.
    pub gates_fired_this_call: i32,
    /// Most recent `tool_use_id` seen by the router. Used to detect a new
    /// tool call and reset `gates_fired_this_call`.
    pub last_seen_tool_use_id: String,
    /// Name of the most recent `P2Advise` gate that fired during `PreToolUse`.
    /// Read by `post_tool` to inject `[ADVISORY_RECOVERY:<gate>]` context
    /// so the next turn auto-heals instead of permission-seeking.
    /// Cleared after one `PostToolUse` emission. SOURCE:
    /// roadmap.unit.agent-feedback-loop.
    pub last_advisory_gate: String,
    /// Mechanical fix text paired with `last_advisory_gate`. Injected into
    /// `PostToolUse` additionalContext so the next turn applies it without
    /// asking the user.
    pub last_advisory_fix: String,
    /// `turn_count` at the most recent Stop attempt — paired with
    /// `last_progress_snapshot_writes` so `has_progress_since_last_stop()`
    /// can detect whether new code-write turns occurred between Stop attempts.
    /// 0 = never snapshotted. SOURCE: roadmap.unit.stop-gate-progress-reset
    /// (rca.stop-breaker-no-progress-reset, 2026-05-23) — multi-wave plans hit
    /// the bounded re-block cap mid-flight because the breaker counts
    /// attempts, not stalls. The reset closes the AWS/Fowler circuit-breaker
    /// success-edge that this breaker was missing.
    pub last_progress_snapshot_turn: i32,
    /// `last_write_turn` value at the most recent Stop attempt. When the
    /// current `last_write_turn` exceeds this snapshot, the agent edited
    /// code since the last Stop → real progress → breaker resets to 0.
    /// SOURCE: AWS Builders' Library circuit-breaker pattern (Closed→Open
    /// requires *consecutive* failures; success resets the count).
    pub last_progress_snapshot_writes: i32,
    /// `last_db_write_turn` value at the most recent Stop attempt. Closing a
    /// kanban card (`kavach db status-update`) is real loop progress but is a
    /// DB write, not a file Write — so the breaker's progress check must count
    /// it too, else a "close card -> dispatch next -> stop" cycle reads as a
    /// no-progress spin and the loop dies after 3 productive card-closes.
    /// SOURCE: rca.stop-breaker-db-progress-blind (2026-06-02) — sibling to
    /// rca.stop-breaker-no-progress-reset; same circuit-breaker success-edge,
    /// widened to the DB-write progress signal.
    pub last_progress_snapshot_db_writes: i32,
    /// Files-modified count at time of most recent code-reviewer completion.
    /// When `files_modified.len() <= last_review_files_count`, the review-gate
    /// at `completion_guard.rs::check_review_isolation` skips the `REVIEW_GATE`
    /// injection — review already covers the current diff.
    /// SOURCE: `decision:rca.review_gate_overfires` (2026-05-10).
    pub last_review_files_count: usize,
    /// Unix timestamp (seconds) of most recent code-reviewer completion.
    /// 0 = never reviewed this session. Used for staleness if file count grows
    /// beyond `last_review_files_count`.
    pub last_review_at: i64,
    /// Classified failure type: transient, `not_found`, permission, validation.
    /// Enables stop gate to make smarter retry/block/skip decisions.
    pub failure_type: String,
    pub required_skills: Vec<String>,
    pub invoked_skills: Vec<String>,
    pub research_topic: String,
    pub last_subagent_turn: i32,
    /// True when a subagent returned actionable results (fix strategies, file paths)
    /// that the parent has not yet acted on. Cleared by post-write gate.
    pub subagent_action_pending: bool,
    /// Turn when `subagent_action_pending` was set.
    pub subagent_action_turn: i32,
    pub recent_commands: Vec<String>,
    /// Files modified without corresponding test coverage (persistent across turns).
    pub test_files_pending: Vec<String>,
    /// Number of times test reminder has been injected (for escalation).
    pub test_nudge_count: i32,
    /// Last API error type from `StopFailure` event.
    pub last_api_error: String,
    /// Cumulative API error count in this session.
    pub api_error_count: i32,
    /// Count of lifestyle advice phrases detected (sleep, break, etc).
    /// Advisory metric, not blocking. Tracks hygiene directive frequency.
    pub lifestyle_advice_count: i32,
    /// ISO timestamp of last API error for cooldown tracking.
    pub last_api_error_time: String,
    /// Critical decisions/blockers/discoveries that MUST survive compaction.
    /// Never summarized — injected verbatim into post-compact context.
    pub case_facts: Vec<String>,
    /// Intent risk level from intent gate (low/medium/high).
    pub intent_risk: String,
    /// Count of files read by current subagent pass (attention dilution tracking).
    pub subagent_files_read: i32,
    /// Turn when a file was last written/edited. Used by `completion_guard`
    /// to skip review isolation when recent turns were read-only.
    pub last_write_turn: i32,
    /// Turn at which the USER issued a directive (set by the intent gate on every
    /// `UserPromptSubmit`). When this equals `turn_count` at a Stop, the user is
    /// STEERING this turn -> the stop gate must NOT dispatch a DIFFERENT kanban
    /// card over the user's live instruction (the user-focus override). The
    /// autonomous loop resumes only on a stop where the user did NOT just speak.
    pub user_directive_turn: i32,
    /// User explicitly confirmed creating a new package manifest in this workspace.
    /// Set by intent gate when user says "create new crate", "yes proceed", etc.
    /// Cleared after the Write succeeds (post-write gate).
    pub new_crate_confirmed: bool,
    /// True when /arch skill was invoked this turn (algorithm research).
    /// Pre-write algo guard reads this — resets to false at turn boundary.
    pub algo_hunter_invoked: bool,
    /// Crate/package names currently under a running `cargo test` / `cargo nextest` command.
    /// Pre-tool-bash guard blocks duplicate test runs on the same crate.
    /// Cleared by post-tool-bash when the command completes.
    pub active_test_crates: Vec<String>,
    /// Turn when `kavach db write` was last called with substantive content.
    /// Stop gate uses this to enforce db write every 5 turns.
    pub last_db_write_turn: i32,
    /// Number of WebSearch/WebFetch calls logged since the last intent reset.
    /// Evidence-chain gate reads this to verify research preceded a Write.
    pub websearch_count_since_intent: i32,
    /// Turn on which the current intent window was opened (intent gate fires).
    /// Used by the evidence-chain gate to scope the correlated block.
    pub intent_set_turn: i32,
    /// True when the [`THINK_FIRST`] advisory has been injected for the current intent window.
    /// Elicitation gate sets this — resets on intent window reset.
    pub think_first_injected: bool,
    /// Files modified in the current user prompt turn (reset on `UserPromptSubmit`).
    /// Surgical guard reads this to detect wide-scope edits.
    pub files_modified_this_turn: Vec<String>,
    /// Files for which LSP diagnostics have been observed this session.
    /// Populated by `PostToolUse` on LSP tool calls (`mcp__cclsp`__*, native LSP tool),
    /// checked by §LSP-FIRST `PreToolUse` gate on Edit/Write/MultiEdit.
    /// SOURCE: ~/.claude/CLAUDE.md §LSP-FIRST — "every edit must follow a
    /// diagnostic read" (claude.com/plugins/rust-analyzer-lsp).
    pub lsp_diag_seen: Vec<String>,
    /// True once the `RESEARCH_PENDING` advisory has been injected for the current intent window.
    /// Pre-tool gate sets this on first advisory — suppresses repeats until intent resets.
    pub research_advisory_sent: bool,
    /// True when /arch skill was invoked this turn.
    /// Pre-write arch guard reads this — resets to false at turn boundary.
    pub arch_skill_invoked: bool,
    // ARCH: CircuitBreakerState
    // PATTERN: circuit_breaker
    // SCOPE: session (per-session state tracking)
    // DECISION: HashMap<category, count> for O(1) lookup/increment
    // FAILURE_MODE: If session corrupt, all circuits reset (fail-open for availability)
    // CAP: AP — availability over consistency (allow work to proceed)
    // SEARCHED: 2026-04
    /// Map of gate category → block count this session. Circuit breaker reads this.
    /// O(1) lookup per gate check. Memory bounded by ~30 gate categories.
    pub gate_block_counts: HashMap<String, i32>,
    /// Maximum blocks per gate category before circuit breaker trips (force-allow).
    /// Default: 3. Per Meta-Harness research: complex verification degrades performance.
    pub gate_circuit_breaker_threshold: i32,
    /// Categories currently in "tripped" state (force-allow with advisory).
    /// Cleared on session end or explicit reset.
    pub tripped_gate_categories: Vec<String>,

    // ARCH: PhaseGatedEnforcement — SDLC phase-based gate activation
    // PATTERN: phase_gate | SCOPE: session | CAP: AP | SEARCHED: 2026-04
    // Per Stanford Meta-Harness: harness sequences enforcement for depth over breadth.
    // Phases: PLAN → IMPLEMENT → TEST → HARDEN. Gates activate per phase.
    /// Current development phase: "PLAN", "IMPLEMENT", "TEST", "HARDEN".
    /// Gates activate based on this phase. Default: "PLAN".
    pub current_phase: String,
    /// Turn when current phase started. Used for phase duration tracking.
    pub phase_start_turn: i32,

    // ARCH: IterationScopeEnforcement — one file at a time, full depth
    // PATTERN: iteration_scope | SCOPE: file | CAP: AP | SEARCHED: 2026-04
    // Pre-write gate blocks writes to files other than current iteration file.
    /// The ONE file currently being worked on. Empty = no iteration active.
    /// Pre-write gate blocks writes to other files when this is set.
    pub current_iteration_file: String,
    /// Files completed in current phase. Cleared on phase transition.
    pub iteration_files_done: Vec<String>,

    // ARCH: PhaseCompletionTracking — per-phase Definition of Done tracking
    // PATTERN: dod_tracking | SCOPE: phase | CAP: AP | SEARCHED: 2026-04
    /// Files that passed PLAN phase (spec committed + research evidence).
    pub plan_done_files: Vec<String>,
    /// Files that passed IMPLEMENT phase (compiles + security gates).
    pub implement_done_files: Vec<String>,
    /// Files that passed TEST phase (tests exist + pass).
    pub test_done_files: Vec<String>,
    /// Files that passed HARDEN phase (all gates clean).
    pub harden_done_files: Vec<String>,

    // ARCH: KanbanSequenceEnforcement — chronological task ordering
    // PATTERN: kanban_sequence | SCOPE: project | CAP: AP | SEARCHED: 2026-04
    /// Currently active kanban card key. Stop gate blocks until marked done.
    pub current_kanban_card: String,
    /// Runnable card count for `kanban:empty` loop target; `None` = uncensused (fail-closed).
    pub loop_kanban_runnable: Option<u64>,
    /// Kanban cards that cannot start yet (blocked by prior cards).
    pub blocked_cards: Vec<String>,
    /// The user's explicit pinned scope (CLAUDE.md §FOCUS — the SUPREME
    /// directive). Empty = no active focus (kanban-drain rules apply). When
    /// non-empty the Stop gate must NOT pull to an unrelated kanban card:
    /// user intent OUTRANKS the queue. Set deterministically from an
    /// explicit `FOCUS:` prompt marker (never inferred); cleared by
    /// `FOCUS:CLEAR`.
    pub user_focus: String,

    // ARCH: GoalOrientedLoop — persist until goal achieved
    // PATTERN: goal_loop | SCOPE: session | CAP: AP | SEARCHED: 2026-04
    // Per Ralph AI pattern: iterate until every requirement addressed.
    // Stop gate blocks unless goal_achieved OR user explicitly stops.
    /// User's goal extracted from prompt. What "done" looks like.
    /// Set by intent gate. Cleared when goal achieved or new prompt.
    pub goal_state: String,
    /// Legacy self-asserted completion flag. Retained ONLY as a fallback for
    /// goals declared without an oracle. Prefer `goal_receipt_pass`, which is
    /// proof-backed. SOURCE: roadmap.unit.goal-oracle-workflow.
    pub goal_achieved: bool,
    /// True ONLY when a verified oracle receipt (a `gate_loop_receipt` row with
    /// `oracle_result == "pass"`) exists for the active goal. This is the
    /// proof-gated completion signal — the stop gate trusts THIS, not the
    /// self-asserted `goal_achieved`. A hallucinated "done" cannot set it.
    pub goal_receipt_pass: bool,
    /// RLAIF (Reinforcement Learning from AI Feedback) verdict for the active
    /// goal: `Some(true)` = the AI judged the work a net advance, `Some(false)` =
    /// a net regression, `None` = no AI judgment this session. Read ONLY when the
    /// mechanical `goal_receipt_pass` is false (no machine receipt) — it fills the
    /// reward blind spot where the 3-witness oracle would otherwise abstain, so
    /// the bandit keeps learning off AI feedback. The mechanical receipt always
    /// wins; this never overrides ground truth. SOURCE: kavach
    /// `decision.arch.harness-rl.design-2026-06-05` (Bai et al. 2022, RLAIF).
    pub ai_verdict: Option<bool>,
    /// Turn when `goal_state` was last set. For staleness detection.
    pub goal_set_turn: i32,

    // ARCH: AutonomousLoopEnforcement — harness engineering loop-until-complete
    // PATTERN: pev_loop (Plan-Execute-Verify) | SCOPE: session | CAP: AP | SEARCHED: 2026-05
    // SOURCE: martinfowler.com/articles/harness-engineering.html
    // Stop hook blocks until loop target reached. Auto-advances phases on DoD met.
    /// True when autonomous loop is active. Stop hook enforces continuation.
    pub loop_active: bool,
    /// Target condition for loop termination: "phase:TEST", "kanban:empty", "goal".
    pub loop_target: String,
    /// Current loop iteration count. For observability and runaway detection.
    pub loop_iteration: i32,
    /// Maximum loop iterations before forced stop (safety valve). Default: 50.
    pub loop_max_iterations: i32,
    /// Turn when loop was started. For duration tracking.
    pub loop_start_turn: i32,

    // ARCH: MultiTurnRcaTracking — persist [RCA] block recognition across turns
    // PATTERN: state_drift_fix | SCOPE: intent_window | CAP: AP | SEARCHED: 2026-05
    // SOURCE: tianpan.co/blog/2026-04-17-multi-turn-session-state-collapse — turn-local
    //   buffers cause 39% perf drop; persistent intent-scoped state recovers signal.
    // SOURCE: code.claude.com/docs/en/hooks — PreToolUse only sees current turn,
    //   session state must persist multi-turn signals.
    /// True when an [RCA] block was recognized in assistant text within the
    /// current intent window. Cleared on intent reset or risk drop to low.
    pub rca_block_present: bool,
    /// Turn when `rca_block_present` was set. For staleness detection.
    pub rca_set_turn: i32,

    // ARCH: BountyScanSignatureCache — skip cargo subprocess scans when inputs
    // unchanged since last clean scan. Stop hook latency: 1.7s -> <10ms hot path.
    /// Signature: "{`lock_mtime}:{toml_mtime}:{rs_count`}". Empty = no prior scan.
    pub bounty_scan_signature: String,
    /// True when last scan returned clean (no findings).
    pub bounty_scan_clean: bool,

    // ARCH: SubagentBlastTracking — track cumulative blast radius across subagents
    // PATTERN: blast_radius | SCOPE: session | CAP: AP | SEARCHED: 2026-05
    // SOURCE: github.com/nousresearch/hermes-agent — persistent memory tracks effects
    // SOURCE: brainblend-ai.github.io/atomic-agents/ — schema validation for safety
    // When cumulative blast exceeds threshold, auto-escalate gate severity P1→P0.
    /// Files written by all subagents this session (cumulative, not reset per agent).
    pub subagent_files_written: Vec<String>,
    /// External APIs called by subagents (URLs, endpoints).
    pub subagent_external_apis: Vec<String>,
    /// True when any subagent performed a database mutation.
    pub subagent_db_mutations: bool,
    /// Tools denied to subagents (inherited from parent context + accumulated).
    /// `SubagentStart` gate propagates this to child agent context.
    pub subagent_denied_tools: Vec<String>,
    /// Blast radius threshold for auto-escalation (files count). Default: 10.
    pub blast_escalation_threshold: i32,
    /// True when blast threshold exceeded — gates escalate P1→P0.
    pub blast_escalated: bool,

    // ARCH: CursorTurnShadowRelay — per-turn context dropped on Cursor allow path
    // PATTERN: session_relay | SCOPE: session | CAP: AP | SEARCHED: 2026-06
    /// Compact per-turn shadow (~600–800 bytes) replayed via `preToolUse` `agent_message`.
    pub turn_shadow: String,
    /// True until the first `pre_tool`/`pre_write` flush consumes the shadow.
    pub turn_shadow_pending: bool,
    /// FIFO post-tool advisories (max 3) merged into the next relay flush.
    pub pending_advisories: Vec<String>,
    /// Last card verify summary for `[REWARD:last]` stop followup.
    pub last_reward_summary: String,
    /// Session verify passes counted for `[REWARD:stats]`.
    pub reward_session_pass: i32,
    /// Session verify attempts counted for `[REWARD:stats]`.
    pub reward_session_total: i32,
}

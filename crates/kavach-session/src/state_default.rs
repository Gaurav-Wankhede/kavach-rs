use std::collections::HashMap;

use chrono::Datelike;

use crate::paths::today;
use crate::state::{
    DEFAULT_SUBAGENT_MAX_OUTPUT, DEFAULT_SUBAGENT_TOTAL_CAP, DEFAULT_TOKEN_BUDGET, SessionState,
};

fn dynamic_training_cutoff() -> String {
    let year = chrono::Utc::now().year().cast_unsigned();
    format!("{}-01", year.saturating_sub(1))
}

impl Default for SessionState {
    #[expect(
        clippy::too_many_lines,
        reason = "linear struct initialization with 130+ fields"
    )]
    fn default() -> Self {
        Self {
            id: String::new(),
            session_id: String::new(),
            today: today(),
            project: String::new(),
            work_dir: String::new(),
            training_cutoff: dynamic_training_cutoff(),
            research_done: false,
            research_topics: Vec::new(),
            memory_queried: false,
            post_compact: false,
            compacted_at: String::new(),
            compact_count: 0,
            turn_count: 0,
            last_reinforce_turn: 0,
            reinforce_every_n: 15,
            current_task: String::new(),
            task_status: String::new(),
            files_modified: Vec::new(),
            tasks_created: 0,
            tasks_completed: 0,
            task_list_id: String::new(),
            intent_type: String::new(),
            intent_domain: String::new(),
            intent_skills: Vec::new(),
            specs_injected: Vec::new(),
            token_budget_total: DEFAULT_TOKEN_BUDGET,
            token_budget_used: 0,
            context_phase: "early".into(),
            active_subagents: 0,
            subagent_outputs: HashMap::new(),
            subagent_max_output: DEFAULT_SUBAGENT_MAX_OUTPUT,
            subagent_total_cap: DEFAULT_SUBAGENT_TOTAL_CAP,
            team_name: String::new(),
            team_members: Vec::new(),
            active_teammates: 0,
            model_id: String::new(),
            modules_injected: Vec::new(),
            last_failure_tool: String::new(),
            last_failure_turn: 0,
            failure_block_count: 0,
            stop_reblock_count: 0,
            gates_fired_this_turn: 0,
            gates_fired_this_call: 0,
            last_seen_tool_use_id: String::new(),
            last_advisory_gate: String::new(),
            last_advisory_fix: String::new(),
            last_progress_snapshot_turn: 0,
            last_progress_snapshot_writes: 0,
            last_progress_snapshot_db_writes: 0,
            last_review_files_count: 0,
            last_review_at: 0,
            failure_type: String::new(),
            required_skills: Vec::new(),
            invoked_skills: Vec::new(),
            research_topic: String::new(),
            last_subagent_turn: 0,
            subagent_action_pending: false,
            subagent_action_turn: 0,
            recent_commands: Vec::new(),
            test_files_pending: Vec::new(),
            test_nudge_count: 0,
            case_facts: Vec::new(),
            intent_risk: "medium".into(),
            subagent_files_read: 0,
            last_write_turn: 0,
            user_directive_turn: 0,
            new_crate_confirmed: false,
            algo_hunter_invoked: false,
            active_test_crates: Vec::new(),
            last_db_write_turn: 0,
            websearch_count_since_intent: 0,
            intent_set_turn: 0,
            think_first_injected: false,
            files_modified_this_turn: Vec::new(),
            tdd_red_units: Vec::new(),
            lsp_diag_seen: Vec::new(),
            research_advisory_sent: false,
            arch_skill_invoked: false,
            // ARCH: CircuitBreakerDefaults — see state.rs for full design rationale
            // PATTERN: circuit_breaker | SCOPE: session | CAP: AP | SEARCHED: 2026-04
            gate_block_counts: HashMap::new(),
            gate_circuit_breaker_threshold: 3,
            tripped_gate_categories: Vec::new(),

            // ARCH: PhaseGatedDefaults — see state.rs for full design rationale
            // PATTERN: phase_gate | SCOPE: session | CAP: AP | SEARCHED: 2026-04
            current_phase: "PLAN".into(),
            phase_start_turn: 0,

            // ARCH: IterationScopeDefaults
            // PATTERN: iteration_scope | SCOPE: file | CAP: AP | SEARCHED: 2026-04
            current_iteration_file: String::new(),
            iteration_files_done: Vec::new(),

            // ARCH: PhaseCompletionDefaults
            // PATTERN: dod_tracking | SCOPE: phase | CAP: AP | SEARCHED: 2026-04
            plan_done_files: Vec::new(),
            implement_done_files: Vec::new(),
            test_done_files: Vec::new(),
            harden_done_files: Vec::new(),

            // ARCH: KanbanSequenceDefaults
            // PATTERN: kanban_sequence | SCOPE: project | CAP: AP | SEARCHED: 2026-04
            current_kanban_card: String::new(),
            loop_kanban_runnable: None,
            blocked_cards: Vec::new(),
            user_focus: String::new(),

            // ARCH: GoalOrientedLoopDefaults
            // PATTERN: goal_loop | SCOPE: session | CAP: AP | SEARCHED: 2026-04
            goal_state: String::new(),
            goal_achieved: false,
            goal_receipt_pass: false,
            ai_verdict: None,
            goal_set_turn: 0,

            // ARCH: AutonomousLoopDefaults
            // PATTERN: pev_loop | SCOPE: session | CAP: AP | SEARCHED: 2026-05
            // SOURCE: martinfowler.com/articles/harness-engineering.html
            loop_active: false,
            loop_target: String::new(),
            loop_iteration: 0,
            loop_max_iterations: 50,
            loop_start_turn: 0,
            rca_block_present: false,
            rca_set_turn: 0,
            bounty_scan_signature: String::new(),
            bounty_scan_clean: false,

            // ARCH: SubagentBlastDefaults — see state.rs for full design rationale
            // PATTERN: blast_radius | SCOPE: session | CAP: AP | SEARCHED: 2026-05
            // SOURCE: github.com/nousresearch/hermes-agent — persistent memory
            subagent_files_written: Vec::new(),
            subagent_external_apis: Vec::new(),
            subagent_db_mutations: false,
            subagent_denied_tools: Vec::new(),
            blast_escalation_threshold: 10,
            blast_escalated: false,
            turn_shadow: String::new(),
            turn_shadow_pending: false,
            pending_advisories: Vec::new(),
            last_reward_summary: String::new(),
            reward_session_pass: 0,
            reward_session_total: 0,
        }
    }
}

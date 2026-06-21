use crate::load::split_csv;
use crate::state::{
    DEFAULT_SUBAGENT_MAX_OUTPUT, DEFAULT_SUBAGENT_TOTAL_CAP, DEFAULT_TOKEN_BUDGET, SessionState,
};

fn set_csv(target: &mut Vec<String>, value: &str) {
    if !value.is_empty() {
        *target = split_csv(value);
    }
}

/// Safe i32 parse — kavach eats its own dogfood (no `unwrap_or` in production).
fn pi32(value: &str, default: i32) -> i32 {
    value.parse().map_or(default, |v| v)
}

#[expect(
    clippy::too_many_lines,
    reason = "linear dispatcher for session state field parsing"
)]
#[expect(
    clippy::redundant_pub_crate,
    reason = "crate-internal, used by load.rs"
)]
pub(crate) fn parse_field(state: &mut SessionState, key: &str, value: &str, in_files: &mut bool) {
    match key {
        "id" => state.id = value.into(),
        "today" => state.today = value.into(),
        "project" => state.project = value.into(),
        "workdir" => state.work_dir = value.into(),
        "research" | "research_done" => state.research_done = value == "true",
        "research_topics" => set_csv(&mut state.research_topics, value),
        "memory" => state.memory_queried = value == "true",
        "cutoff" => state.training_cutoff = value.into(),
        "post_compact" => state.post_compact = value == "true",
        "compacted_at" => state.compacted_at = value.into(),
        "compact_count" => state.compact_count = pi32(value, 0),
        "turn_count" => state.turn_count = pi32(value, 0),
        "last_reinforce_turn" => state.last_reinforce_turn = pi32(value, 0),
        "reinforce_every_n" => state.reinforce_every_n = pi32(value, 15),
        "tasks_created" => state.tasks_created = pi32(value, 0),
        "tasks_completed" => state.tasks_completed = pi32(value, 0),
        "session_id" => state.session_id = value.into(),
        "task" => state.current_task = value.into(),
        "task_status" => state.task_status = value.into(),
        "task_list_id" => state.task_list_id = value.into(),
        "files[]" => {
            *in_files = true;
            if !value.is_empty() {
                state.files_modified.push(value.into());
            }
        }
        "type" => {
            if state.intent_type.is_empty() {
                state.intent_type = value.into();
            }
        }
        "domain" => {
            if state.intent_domain.is_empty() {
                state.intent_domain = value.into();
            }
        }
        "skills" => {
            if !value.is_empty() && state.intent_skills.is_empty() {
                state.intent_skills = split_csv(value);
            }
        }
        "token_budget_total" => state.token_budget_total = pi32(value, DEFAULT_TOKEN_BUDGET),
        "token_budget_used" => state.token_budget_used = pi32(value, 0),
        "context_phase" => state.context_phase = value.into(),
        "active_subagents" => state.active_subagents = pi32(value, 0),
        "subagent_max_output" => {
            state.subagent_max_output = pi32(value, DEFAULT_SUBAGENT_MAX_OUTPUT);
        }
        "subagent_total_cap" => state.subagent_total_cap = pi32(value, DEFAULT_SUBAGENT_TOTAL_CAP),
        "team_name" => state.team_name = value.into(),
        "team_members" => set_csv(&mut state.team_members, value),
        "active_teammates" => state.active_teammates = pi32(value, 0),
        "model_id" => state.model_id = value.into(),
        "last_failure_tool" => state.last_failure_tool = value.into(),
        "last_failure_turn" => state.last_failure_turn = pi32(value, 0),
        "failure_block_count" => state.failure_block_count = pi32(value, 0),
        "stop_reblock_count" => state.stop_reblock_count = pi32(value, 0),
        "last_progress_snapshot_turn" => state.last_progress_snapshot_turn = pi32(value, 0),
        "last_progress_snapshot_writes" => state.last_progress_snapshot_writes = pi32(value, 0),
        "last_progress_snapshot_db_writes" => {
            state.last_progress_snapshot_db_writes = pi32(value, 0);
        }
        "last_review_files_count" => {
            state.last_review_files_count = usize::try_from(pi32(value, 0).max(0)).unwrap_or(0);
        }
        "last_review_at" => state.last_review_at = value.parse().unwrap_or(0),
        "failure_type" => state.failure_type = value.into(),
        "specs_injected" => set_csv(&mut state.specs_injected, value),
        "modules_injected" => set_csv(&mut state.modules_injected, value),
        "required_skills" => set_csv(&mut state.required_skills, value),
        "invoked_skills" => set_csv(&mut state.invoked_skills, value),
        "research_topic" => state.research_topic = value.into(),
        "last_subagent_turn" => state.last_subagent_turn = pi32(value, 0),
        "subagent_action_pending" => state.subagent_action_pending = value == "true",
        "subagent_action_turn" => state.subagent_action_turn = pi32(value, 0),
        "recent_commands" => set_csv(&mut state.recent_commands, value),
        "test_files_pending" => set_csv(&mut state.test_files_pending, value),
        "test_nudge_count" => state.test_nudge_count = pi32(value, 0),
        "last_api_error" => state.last_api_error = value.into(),
        "api_error_count" => state.api_error_count = pi32(value, 0),
        "lifestyle_advice_count" => state.lifestyle_advice_count = pi32(value, 0),
        "last_api_error_time" => state.last_api_error_time = value.into(),
        "intent_risk" => state.intent_risk = value.into(),
        "last_write_turn" => state.last_write_turn = pi32(value, 0),
        "user_directive_turn" => state.user_directive_turn = pi32(value, 0),
        "last_db_write_turn" => state.last_db_write_turn = pi32(value, 0),
        "subagent_files_read" => state.subagent_files_read = pi32(value, 0),
        "new_crate_confirmed" => state.new_crate_confirmed = value == "true",
        "algo_hunter_invoked" => state.algo_hunter_invoked = value == "true",
        "websearch_count_since_intent" => state.websearch_count_since_intent = pi32(value, 0),
        "intent_set_turn" => state.intent_set_turn = pi32(value, 0),
        "think_first_injected" => state.think_first_injected = value == "true",
        "research_advisory_sent" => state.research_advisory_sent = value == "true",
        "arch_skill_invoked" => state.arch_skill_invoked = value == "true",
        // ARCH: CircuitBreakerParsing — field parsing for circuit breaker state
        // PATTERN: circuit_breaker | SCOPE: session | CAP: AP | SEARCHED: 2026-04
        "gate_circuit_breaker_threshold" => state.gate_circuit_breaker_threshold = pi32(value, 3),
        "tripped_gate_categories" => set_csv(&mut state.tripped_gate_categories, value),
        // FIX [state_drift] — reverse of serialize_extras.rs gate_block_counts
        // emit: split on ',', then on LAST '=' (count is the suffix), then
        // percent-decode the name so colon/comma/equals embedded in a
        // category name survive the round-trip.
        "gate_block_counts" => {
            state.gate_block_counts.clear();
            for entry in value.split(',').filter(|e| !e.is_empty()) {
                if let Some(eq) = entry.rfind('=') {
                    let (raw_name, count_str) = entry.split_at(eq);
                    let count = pi32(count_str.trim_start_matches('='), 0);
                    let name = raw_name
                        .replace("%3D", "=")
                        .replace("%2C", ",")
                        .replace("%25", "%");
                    if !name.is_empty() {
                        state.gate_block_counts.insert(name, count);
                    }
                }
            }
        }
        "files_modified_this_turn" => {
            state.files_modified_this_turn = value
                .split(',')
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect();
        }
        "tdd_red_units" => {
            state.tdd_red_units = value
                .split(',')
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect();
        }
        // FIX [contract_violation] reviewer BLOCK-J — see serialize_extras.rs
        // [LSP_FIRST] section. Without this parse arm, the §LSP-FIRST gate's
        // producer bookkeeping (post_tool_lsp.rs) would reset every session
        // and the consumer (pre_write_lsp_first.rs) would re-emit advisories
        // for already-diagnosed files.
        "lsp_diag_seen" => {
            state.lsp_diag_seen = value
                .split(',')
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect();
        }
        "active_test_crates" => {
            state.active_test_crates = value
                .split(',')
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect();
        }
        // ARCH: PhaseGatedParsing — field parsing for phase enforcement state
        // PATTERN: phase_gate | SCOPE: session | CAP: AP | SEARCHED: 2026-04
        "current_phase" => state.current_phase = value.into(),
        "phase_start_turn" => state.phase_start_turn = pi32(value, 0),
        "current_iteration_file" => state.current_iteration_file = value.into(),
        "iteration_files_done" => set_csv(&mut state.iteration_files_done, value),
        // ARCH: PhaseCompletionParsing — field parsing for per-phase DoD tracking
        // PATTERN: dod_tracking | SCOPE: phase | CAP: AP | SEARCHED: 2026-04
        "plan_done_files" => set_csv(&mut state.plan_done_files, value),
        "implement_done_files" => set_csv(&mut state.implement_done_files, value),
        "test_done_files" => set_csv(&mut state.test_done_files, value),
        "harden_done_files" => set_csv(&mut state.harden_done_files, value),
        // ARCH: KanbanSequenceParsing — field parsing for kanban ordering state
        // PATTERN: kanban_sequence | SCOPE: project | CAP: AP | SEARCHED: 2026-04
        "current_kanban_card" => state.current_kanban_card = value.into(),
        "user_focus" => state.user_focus = value.into(),
        "blocked_cards" => set_csv(&mut state.blocked_cards, value),
        // ARCH: GoalOrientedLoopParsing — field parsing for goal state
        // PATTERN: goal_loop | SCOPE: session | CAP: AP | SEARCHED: 2026-04
        "goal_state" => state.goal_state = value.into(),
        "goal_achieved" => state.goal_achieved = value == "true",
        "goal_receipt_pass" => state.goal_receipt_pass = value == "true",
        "ai_verdict" => state.ai_verdict = Some(value == "true"),
        "goal_set_turn" => state.goal_set_turn = pi32(value, 0),
        // ARCH: AutonomousLoopParsing — field parsing for loop state
        // PATTERN: pev_loop | SCOPE: session | CAP: AP | SEARCHED: 2026-05
        // SOURCE: martinfowler.com/articles/harness-engineering.html
        "loop_active" => state.loop_active = value == "true",
        "loop_target" => state.loop_target = value.into(),
        "loop_iteration" => state.loop_iteration = pi32(value, 0),
        "loop_max_iterations" => state.loop_max_iterations = pi32(value, 50),
        "loop_start_turn" => state.loop_start_turn = pi32(value, 0),
        // ARCH: MultiTurnRcaTracking — see state.rs
        "rca_block_present" => state.rca_block_present = value == "true",
        "bounty_scan_signature" => state.bounty_scan_signature = value.into(),
        "bounty_scan_clean" => state.bounty_scan_clean = value == "true",
        "rca_set_turn" => state.rca_set_turn = pi32(value, 0),
        // ARCH: SubagentBlastParsing — field parsing for blast radius tracking
        // PATTERN: blast_radius | SCOPE: session | CAP: AP | SEARCHED: 2026-05
        // SOURCE: github.com/nousresearch/hermes-agent — persistent memory
        "subagent_files_written" => set_csv(&mut state.subagent_files_written, value),
        "subagent_external_apis" => set_csv(&mut state.subagent_external_apis, value),
        "subagent_db_mutations" => state.subagent_db_mutations = value == "true",
        "subagent_denied_tools" => set_csv(&mut state.subagent_denied_tools, value),
        "blast_escalation_threshold" => state.blast_escalation_threshold = pi32(value, 10),
        "blast_escalated" => state.blast_escalated = value == "true",
        "turn_shadow_pending" => state.turn_shadow_pending = value == "true",
        "turn_shadow" => state.turn_shadow = value.replace("\\n", "\n"),
        "pending_advisories" => {
            state.pending_advisories = value
                .split("\\n")
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect();
        }
        "last_reward_summary" => state.last_reward_summary = value.into(),
        "reward_session_pass" => state.reward_session_pass = pi32(value, 0),
        "reward_session_total" => state.reward_session_total = pi32(value, 0),
        _ => parse_output_field(state, key, value),
    }
}

fn parse_output_field(state: &mut SessionState, key: &str, value: &str) {
    if let Some(agent_id) = key.strip_prefix("output:")
        && !agent_id.is_empty()
    {
        let chars: i32 = pi32(value, 0);
        state.subagent_outputs.insert(agent_id.into(), chars);
    }
}

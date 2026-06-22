use crate::save::join_csv;
use crate::serialize::write_kv;
use crate::state::SessionState;
use std::fmt::Write;

impl SessionState {
    #[expect(
        clippy::too_many_lines,
        reason = "linear dispatcher—single turn through session state sections"
    )]
    pub(crate) fn serialize_extras(&self, s: &mut String) {
        const SER_CAP: usize = 500;
        if !self.intent_type.is_empty() {
            s.push_str("[INTENT_BRIDGE]\n");
            write_kv(s, "type", &self.intent_type);
            write_kv(s, "domain", &self.intent_domain);
            if !self.intent_skills.is_empty() {
                write_kv(s, "skills", &join_csv(&self.intent_skills));
            }
            s.push('\n');
        }

        s.push_str("[TOKEN_BUDGET]\n");
        write_kv(
            s,
            "token_budget_total",
            &self.token_budget_total.to_string(),
        );
        write_kv(s, "token_budget_used", &self.token_budget_used.to_string());
        write_kv(s, "context_phase", &self.context_phase);
        s.push('\n');

        s.push_str("[SUBAGENT_TRACKING]\n");
        write_kv(s, "active_subagents", &self.active_subagents.to_string());
        write_kv(
            s,
            "subagent_max_output",
            &self.subagent_max_output.to_string(),
        );
        write_kv(
            s,
            "subagent_total_cap",
            &self.subagent_total_cap.to_string(),
        );
        write_kv(
            s,
            "last_subagent_turn",
            &self.last_subagent_turn.to_string(),
        );
        write_kv(
            s,
            "subagent_action_pending",
            if self.subagent_action_pending {
                "true"
            } else {
                "false"
            },
        );
        write_kv(
            s,
            "subagent_action_turn",
            &self.subagent_action_turn.to_string(),
        );
        // Sort by key for deterministic output; iterate as (id, &chars) pairs
        // so we never re-index the HashMap (eliminates the `vec[i]` style
        // direct lookup the /rust skill flags).
        let mut sorted: Vec<(&String, &i32)> = self.subagent_outputs.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(b.0));
        for (id, chars) in sorted {
            // fmt::Write on String is infallible (String has unbounded growth
            // — `String::write_str` always returns Ok). write! macro directly
            // on String avoids allocating intermediate format buffer.
            writeln!(s, "output:{id}: {chars}").ok();
        }
        s.push('\n');

        if !self.modules_injected.is_empty() {
            s.push_str("[MODULES]\n");
            write_kv(s, "modules_injected", &join_csv(&self.modules_injected));
            s.push('\n');
        }

        if !self.specs_injected.is_empty() {
            s.push_str("[SPECS]\n");
            write_kv(s, "specs_injected", &join_csv(&self.specs_injected));
            s.push('\n');
        }

        if !self.last_failure_tool.is_empty() {
            s.push_str("[FAILURE_TRACKING]\n");
            write_kv(s, "last_failure_tool", &self.last_failure_tool);
            write_kv(s, "last_failure_turn", &self.last_failure_turn.to_string());
            write_kv(
                s,
                "failure_block_count",
                &self.failure_block_count.to_string(),
            );
            if !self.failure_type.is_empty() {
                write_kv(s, "failure_type", &self.failure_type);
            }
            s.push('\n');
        }
        // FIX [state_drift / lost_update] — stop_reblock_count is the
        // pending-work circuit breaker counter. Persist unconditionally in
        // its own section; nesting under [FAILURE_TRACKING] re-coupled disk
        // persistence to last_failure_tool, causing the perpetual-1/3
        // live-lock when no tool failures occurred between Stop attempts.
        // SOURCE: github.com/elastic/elasticsearch/pull/144827
        s.push_str("[STOP_BREAKER]\n");
        write_kv(
            s,
            "stop_reblock_count",
            &self.stop_reblock_count.to_string(),
        );
        // Progress snapshot for the multi-wave-plan reset edge.
        // SOURCE: rca.stop-breaker-no-progress-reset (2026-05-23).
        write_kv(
            s,
            "last_progress_snapshot_turn",
            &self.last_progress_snapshot_turn.to_string(),
        );
        write_kv(
            s,
            "last_progress_snapshot_writes",
            &self.last_progress_snapshot_writes.to_string(),
        );
        write_kv(
            s,
            "last_progress_snapshot_db_writes",
            &self.last_progress_snapshot_db_writes.to_string(),
        );
        s.push('\n');

        // Review-tracking — written only when a review has occurred this session.
        // Bug fix: completion_guard.rs::check_review_isolation was firing on every
        // stop with files_modified.len() >= 5; persisting these lets the gate
        // suppress repeat warnings after a review covers the current diff.
        // SOURCE: decision:rca.review_gate_overfires
        if self.last_review_at > 0 {
            s.push_str("[REVIEW_TRACKING]\n");
            write_kv(
                s,
                "last_review_files_count",
                &self.last_review_files_count.to_string(),
            );
            write_kv(s, "last_review_at", &self.last_review_at.to_string());
            s.push('\n');
        }


        if self.new_crate_confirmed {
            s.push_str("[NEW_CRATE]\n");
            write_kv(s, "new_crate_confirmed", "true");
            s.push('\n');
        }

        if self.algo_hunter_invoked {
            s.push_str("[ALGO_HUNTER]\n");
            write_kv(s, "algo_hunter_invoked", "true");
            s.push('\n');
        }

        if self.arch_skill_invoked {
            s.push_str("[ARCH_GATE]\n");
            write_kv(s, "arch_skill_invoked", "true");
            s.push('\n');
        }

        if self.websearch_count_since_intent > 0 || self.intent_set_turn > 0 {
            s.push_str("[EVIDENCE_CHAIN]\n");
            write_kv(
                s,
                "websearch_count_since_intent",
                &self.websearch_count_since_intent.to_string(),
            );
            write_kv(s, "intent_set_turn", &self.intent_set_turn.to_string());
            s.push('\n');
        }

        if self.think_first_injected {
            s.push_str("[ELICITATION]\n");
            write_kv(s, "think_first_injected", "true");
            s.push('\n');
        }

        if self.research_advisory_sent {
            s.push_str("[RESEARCH_ADVISORY]\n");
            write_kv(s, "research_advisory_sent", "true");
            s.push('\n');
        }

        if !self.files_modified_this_turn.is_empty() {
            s.push_str("[SURGICAL_TRACKING]\n");
            write_kv(
                s,
                "files_modified_this_turn",
                &self.files_modified_this_turn.join(","),
            );
            s.push('\n');
        }

        if !self.tdd_red_units.is_empty() {
            write_kv(s, "tdd_red_units", &self.tdd_red_units.join(","));
            s.push('\n');
        }

        // FIX [contract_violation] reviewer BLOCK-J — lsp_diag_seen was added
        // to SessionState (commit 729529a) without serialize/parse wiring;
        // the §LSP-FIRST advisory would re-fire every turn because the
        // producer's bookkeeping reset on every session load.
        // SOURCE: ~/.claude/CLAUDE.md §LSP-FIRST + cold reviewer 2026-05.
        if !self.lsp_diag_seen.is_empty() {
            s.push_str("[LSP_FIRST]\n");
            // Cap serialized list at 500 entries to bound state-file growth;
            // matches the LSP_DIAG_SEEN_CAP that pre_write_lsp_first.rs
            // enforces on insert (reviewer FIX-G).
            let tail_start = self.lsp_diag_seen.len().saturating_sub(SER_CAP);
            let slice = self.lsp_diag_seen.get(tail_start..).unwrap_or(&[]);
            let view: Vec<&str> = slice.iter().map(String::as_str).collect();
            write_kv(s, "lsp_diag_seen", &view.join(","));
            s.push('\n');
        }

        if !self.active_test_crates.is_empty() {
            s.push_str("[TEST_RUN_TRACKING]\n");
            write_kv(s, "active_test_crates", &self.active_test_crates.join(","));
            s.push('\n');
        }

        //            {"name":"bincode","reason":"binary blobs hostile to grep"},
        //            {"name":"per-key INI line","reason":"unbounded section bloat"}]
        // TIME: O(n log n) sort (deterministic order) | SPACE: O(n)
        // YEAR: 2026 | SEARCHED: 2026-05 | TRADEOFF: O(n log n) over O(n)
        // accepted for stable round-trip diffs.
        // SOURCE: docs.rs/csv tutorial — escape comma/equals before flat-join.
        //
        // ARCH: CircuitBreakerSerialization — persist circuit breaker state
        // FIX [state_drift / lost_update] — gate_block_counts HashMap was on
        // SessionState but never serialized; record_gate_block counter reset
        // to 0 every save/load cycle, so behavioral breaker never tripped.
        let has_block_counts = !self.gate_block_counts.is_empty();
        if !self.tripped_gate_categories.is_empty()
            || self.gate_circuit_breaker_threshold != 3
            || has_block_counts
        {
            s.push_str("[CIRCUIT_BREAKER]\n");
            write_kv(
                s,
                "gate_circuit_breaker_threshold",
                &self.gate_circuit_breaker_threshold.to_string(),
            );
            if !self.tripped_gate_categories.is_empty() {
                write_kv(
                    s,
                    "tripped_gate_categories",
                    &join_csv(&self.tripped_gate_categories),
                );
            }
            if has_block_counts {
                let mut entries: Vec<String> = self
                    .gate_block_counts
                    .iter()
                    .map(|(k, v)| {
                        let safe = k
                            .replace('%', "%25")
                            .replace(',', "%2C")
                            .replace('=', "%3D");
                        format!("{safe}={v}")
                    })
                    .collect();
                entries.sort();
                write_kv(s, "gate_block_counts", &entries.join(","));
            }
            s.push('\n');
        }

        // ARCH: PhaseGatedSerialization — persist phase enforcement state
        // PATTERN: phase_gate | SCOPE: session | CAP: AP | SEARCHED: 2026-04
        if !self.current_phase.is_empty() && self.current_phase != "PLAN"
            || self.phase_start_turn > 0
            || !self.current_iteration_file.is_empty()
        {
            s.push_str("[PHASE_ENFORCEMENT]\n");
            write_kv(s, "current_phase", &self.current_phase);
            write_kv(s, "phase_start_turn", &self.phase_start_turn.to_string());
            if !self.current_iteration_file.is_empty() {
                write_kv(s, "current_iteration_file", &self.current_iteration_file);
            }
            if !self.iteration_files_done.is_empty() {
                write_kv(
                    s,
                    "iteration_files_done",
                    &join_csv(&self.iteration_files_done),
                );
            }
            s.push('\n');
        }

        // ARCH: PhaseCompletionSerialization — persist per-phase DoD tracking
        // PATTERN: dod_tracking | SCOPE: phase | CAP: AP | SEARCHED: 2026-04
        if !self.plan_done_files.is_empty()
            || !self.implement_done_files.is_empty()
            || !self.test_done_files.is_empty()
            || !self.harden_done_files.is_empty()
        {
            s.push_str("[PHASE_COMPLETION]\n");
            if !self.plan_done_files.is_empty() {
                write_kv(s, "plan_done_files", &join_csv(&self.plan_done_files));
            }
            if !self.implement_done_files.is_empty() {
                write_kv(
                    s,
                    "implement_done_files",
                    &join_csv(&self.implement_done_files),
                );
            }
            if !self.test_done_files.is_empty() {
                write_kv(s, "test_done_files", &join_csv(&self.test_done_files));
            }
            if !self.harden_done_files.is_empty() {
                write_kv(s, "harden_done_files", &join_csv(&self.harden_done_files));
            }
            s.push('\n');
        }

        // ARCH: KanbanSequenceSerialization — persist kanban ordering state
        // PATTERN: kanban_sequence | SCOPE: project | CAP: AP | SEARCHED: 2026-04
        if !self.current_kanban_card.is_empty() || !self.blocked_cards.is_empty() {
            s.push_str("[KANBAN_SEQUENCE]\n");
            if !self.current_kanban_card.is_empty() {
                write_kv(s, "current_kanban_card", &self.current_kanban_card);
            }
            if !self.blocked_cards.is_empty() {
                write_kv(s, "blocked_cards", &join_csv(&self.blocked_cards));
            }
            s.push('\n');
        }

        // §FOCUS — the user's pinned scope OUTRANKS the kanban. Persisted in
        // its own section (not KANBAN_SEQUENCE) because it is the OVERRIDE of
        // the queue, not a member of it.
        if !self.user_focus.is_empty() {
            s.push_str("[USER_FOCUS]\n");
            write_kv(s, "user_focus", &self.user_focus);
            s.push('\n');
        }

        // ARCH: GoalOrientedLoopSerialization — persist goal state
        // PATTERN: goal_loop | SCOPE: session | CAP: AP | SEARCHED: 2026-04
        if !self.goal_state.is_empty()
            || self.goal_achieved
            || self.goal_receipt_pass
            || self.ai_verdict.is_some()
        {
            s.push_str("[GOAL_STATE]\n");
            if !self.goal_state.is_empty() {
                write_kv(s, "goal_state", &self.goal_state);
            }
            write_kv(
                s,
                "goal_achieved",
                if self.goal_achieved { "true" } else { "false" },
            );
            write_kv(
                s,
                "goal_receipt_pass",
                if self.goal_receipt_pass {
                    "true"
                } else {
                    "false"
                },
            );
            if let Some(v) = self.ai_verdict {
                write_kv(s, "ai_verdict", if v { "true" } else { "false" });
            }
            write_kv(s, "goal_set_turn", &self.goal_set_turn.to_string());
            s.push('\n');
        }

        // ARCH: AutonomousLoopSerialization — persist loop state
        // PATTERN: pev_loop | SCOPE: session | CAP: AP | SEARCHED: 2026-05
        // SOURCE: martinfowler.com/articles/harness-engineering.html
        if self.loop_active || self.loop_iteration > 0 {
            s.push_str("[AUTONOMOUS_LOOP]\n");
            write_kv(
                s,
                "loop_active",
                if self.loop_active { "true" } else { "false" },
            );
            if !self.loop_target.is_empty() {
                write_kv(s, "loop_target", &self.loop_target);
            }
            write_kv(s, "loop_iteration", &self.loop_iteration.to_string());
            write_kv(
                s,
                "loop_max_iterations",
                &self.loop_max_iterations.to_string(),
            );
            write_kv(s, "loop_start_turn", &self.loop_start_turn.to_string());
            s.push('\n');
        }

        // ARCH: MultiTurnRcaTracking — see state.rs
        if self.rca_block_present || self.rca_set_turn > 0 {
            s.push_str("[RCA_TRACKING]\n");
            write_kv(
                s,
                "rca_block_present",
                if self.rca_block_present {
                    "true"
                } else {
                    "false"
                },
            );
            if !self.bounty_scan_signature.is_empty() {
                write_kv(s, "bounty_scan_signature", &self.bounty_scan_signature);
            }
            if self.bounty_scan_clean {
                write_kv(s, "bounty_scan_clean", "true");
            }
            write_kv(s, "rca_set_turn", &self.rca_set_turn.to_string());
            s.push('\n');
        }

        // ARCH: SubagentBlastSerialization — persist blast radius tracking
        // PATTERN: blast_radius | SCOPE: session | CAP: AP | SEARCHED: 2026-05
        // SOURCE: github.com/nousresearch/hermes-agent — persistent memory
        if !self.subagent_files_written.is_empty()
            || !self.subagent_external_apis.is_empty()
            || self.subagent_db_mutations
            || !self.subagent_denied_tools.is_empty()
            || self.blast_escalated
        {
            s.push_str("[SUBAGENT_BLAST]\n");
            if !self.subagent_files_written.is_empty() {
                write_kv(
                    s,
                    "subagent_files_written",
                    &join_csv(&self.subagent_files_written),
                );
            }
            if !self.subagent_external_apis.is_empty() {
                write_kv(
                    s,
                    "subagent_external_apis",
                    &join_csv(&self.subagent_external_apis),
                );
            }
            if self.subagent_db_mutations {
                write_kv(s, "subagent_db_mutations", "true");
            }
            if !self.subagent_denied_tools.is_empty() {
                write_kv(
                    s,
                    "subagent_denied_tools",
                    &join_csv(&self.subagent_denied_tools),
                );
            }
            write_kv(
                s,
                "blast_escalation_threshold",
                &self.blast_escalation_threshold.to_string(),
            );
            if self.blast_escalated {
                write_kv(s, "blast_escalated", "true");
            }
            s.push('\n');
        }

        if self.turn_shadow_pending
            || !self.turn_shadow.is_empty()
            || !self.pending_advisories.is_empty()
        {
            s.push_str("[TURN_SHADOW]\n");
            write_kv(
                s,
                "turn_shadow_pending",
                if self.turn_shadow_pending {
                    "true"
                } else {
                    "false"
                },
            );
            if !self.turn_shadow.is_empty() {
                write_kv(s, "turn_shadow", &self.turn_shadow.replace('\n', "\\n"));
            }
            if !self.pending_advisories.is_empty() {
                write_kv(
                    s,
                    "pending_advisories",
                    &self.pending_advisories.join("\\n"),
                );
            }
            s.push('\n');
        }

        if !self.last_reward_summary.is_empty()
            || self.reward_session_pass > 0
            || self.reward_session_total > 0
        {
            s.push_str("[REWARD_TRACKING]\n");
            if !self.last_reward_summary.is_empty() {
                write_kv(s, "last_reward_summary", &self.last_reward_summary);
            }
            if self.reward_session_pass > 0 {
                write_kv(
                    s,
                    "reward_session_pass",
                    &self.reward_session_pass.to_string(),
                );
            }
            if self.reward_session_total > 0 {
                write_kv(
                    s,
                    "reward_session_total",
                    &self.reward_session_total.to_string(),
                );
            }
            s.push('\n');
        }

        self.serialize_enforcement_sections(s);
    }
}

// Tests moved to serialize_tests.rs (test_to_ini, test_to_ini_full_roundtrip)
#[cfg(test)]
#[path = "serialize_tests.rs"]
mod tests;

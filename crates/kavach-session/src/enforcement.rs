use crate::paths::{detect_project, today};
use crate::state::SessionState;
use chrono::Local;

pub(crate) fn generate_session_id(work_dir: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(work_dir.as_bytes());
    hasher.update(Local::now().format("%Y%m%d").to_string().as_bytes());
    let hash = hasher.finalize();
    let hex: String = hash
        .as_bytes()
        .iter()
        .take(16)
        .fold(String::new(), |mut s, b| {
            std::fmt::Write::write_fmt(&mut s, format_args!("{b:02x}")).ok();
            s
        });
    format!("sess_{hex}")
}

impl SessionState {
    /// Create a new session with a unique ID, current date, and project detection.
    #[must_use]
    pub fn new(work_dir: &str) -> Self {
        let id = generate_session_id(work_dir);
        Self {
            id: id.clone(),
            session_id: id,
            today: today(),
            work_dir: work_dir.into(),
            project: detect_project(),
            ..Default::default()
        }
    }

    /// Record which skills are required for this prompt turn.
    pub fn set_required_skills(&mut self, skills: Vec<String>) {
        self.required_skills = skills;
        self.invoked_skills.clear();
        self.save_or_log();
    }

    /// Record a skill invocation atomically.
    /// Uses `atomic_update` to prevent TOCTOU race when parallel Skill hooks fire.
    pub fn record_skill_invoked(&mut self, skill_name: &str) {
        const ARCH_SKILL_NAME: &str = "arch";
        let name = skill_name.to_owned();
        let is_arch = skill_name == ARCH_SKILL_NAME;
        if let Err(e) = self.atomic_update(|s| {
            if !s.invoked_skills.iter().any(|x| x == &name) {
                s.invoked_skills.push(name.clone());
            }
            if is_arch {
                s.arch_skill_invoked = true;
            }
        }) {
            tracing::warn!(error = %e, "record_skill_invoked atomic_update failed; falling back to non-atomic");
            if !self.invoked_skills.iter().any(|s| s == skill_name) {
                self.invoked_skills.push(skill_name.into());
            }
            if is_arch {
                self.arch_skill_invoked = true;
            }
            self.save_or_log();
        }
    }

    /// Get list of missing (not yet invoked) skills.
    #[must_use]
    pub fn missing_skills(&self) -> Vec<&str> {
        self.required_skills
            .iter()
            .filter(|req| !self.invoked_skills.iter().any(|inv| inv == req.as_str()))
            .map(String::as_str)
            .collect()
    }

    /// Set what topic needs research.
    pub fn set_research_topic(&mut self, topic: &str) {
        self.research_topic = topic.into();
        self.save_or_log();
    }

    /// Check if any files are pending test coverage.
    #[must_use]
    pub const fn has_pending_tests(&self) -> bool {
        !self.test_files_pending.is_empty()
    }

    /// Always false — test enforcement is advisory-only, never a hard block.
    /// Hard-blocking caused circular deadlocks when compile errors prevented
    /// cargo test from clearing the gate. Nudges remain active via `post_write`.
    #[must_use]
    pub const fn should_block_for_tests(&self) -> bool {
        false
    }

    /// Clear all pending test files (called after tests are run).
    pub fn clear_test_pending(&mut self) {
        self.test_files_pending.clear();
        self.test_nudge_count = 0;
        self.save_or_log();
    }

    /// Open a new evidence window for the current intent turn.
    /// Called by the intent gate whenever a new user prompt is classified.
    pub fn reset_evidence_window(&mut self) {
        self.websearch_count_since_intent = 0;
        self.intent_set_turn = self.turn_count;
        // Reset per-turn surgical tracking and elicitation flag on each new intent window.
        self.think_first_injected = false;
        self.research_advisory_sent = false;
        self.files_modified_this_turn = Vec::new();
        self.tdd_red_units = Vec::new();
        // Reset RCA tracking — every new intent window requires fresh [RCA] block.
        self.rca_block_present = false;
        self.rca_set_turn = 0;
        self.save_or_log();
    }

    /// Mark that an [RCA] block was recognized in assistant text this turn.
    /// Called by post-tool hooks when scanning the last assistant message.
    /// Persists across turns within the current intent window.
    pub fn mark_rca_present(&mut self) {
        self.rca_block_present = true;
        self.rca_set_turn = self.turn_count;
        self.save_or_log();
    }

    /// True when [RCA] was emitted in this intent window.
    /// Pre-write RCA gate calls this OR scans current message — either satisfies.
    #[must_use]
    pub const fn rca_satisfied(&self) -> bool {
        self.rca_block_present
    }

    /// Record one `WebSearch` or `WebFetch` tool completion in the current window.
    pub fn record_websearch(&mut self) {
        self.websearch_count_since_intent = self.websearch_count_since_intent.saturating_add(1);
        self.save_or_log();
    }

    /// Return true when sufficient research evidence exists before a Write.
    ///
    /// Blocks only when ALL conditions hold simultaneously:
    /// - `intent_type` is "implement" (research required by design)
    /// - no `WebSearch` has fired since the intent was set
    /// - the window is fresh (`intent_set_turn` == `turn_count`, same turn or one
    ///   turn behind — prevents blocking writes in later turns of a multi-turn
    ///   implement flow that already researched on turn N)
    #[must_use]
    pub fn evidence_window_satisfied(&self) -> bool {
        if self.intent_type != "implement" {
            return true;
        }
        if self.websearch_count_since_intent > 0 {
            return true;
        }
        // Allow writes in turns after the intent turn — research may have
        // occurred in the same turn as the intent classification, which is
        // persisted via record_websearch before pre_write fires.
        false
    }

    // ARCH: CircuitBreakerMethods — O(1) operations for gate circuit breaker
    // PATTERN: circuit_breaker | SCOPE: session | CAP: AP | SEARCHED: 2026-04
    // Per Meta-Harness research: complex verification gates degrade performance.
    // Circuit breaker trips after N blocks per category, then force-allows.

    /// Record a gate block for a category. Returns true if circuit should trip.
    /// O(1) lookup + increment via `HashMap`.
    pub fn record_gate_block(&mut self, category: &str) -> bool {
        let count = self.gate_block_counts.entry(category.into()).or_insert(0);
        *count = count.saturating_add(1);
        let prev_count = count.saturating_sub(1);
        let should_trip = *count >= self.gate_circuit_breaker_threshold;
        let pushed_trip =
            should_trip && !self.tripped_gate_categories.contains(&category.to_owned());
        if pushed_trip {
            self.tripped_gate_categories.push(category.into());
        }
        // FIX: [silent_failure/auth_bypass] enforcement.rs:166
        // WHY5: a fail-closed security control must leave NO observable
        //       trip-state when the trip is not durable. Returning `false`
        //       alone was insufficient: `tripped_gate_categories` was already
        //       mutated, so a later `is_gate_tripped()` on the SAME instance
        //       (before any reload) reported an unpersisted trip — the exact
        //       silent circuit-breaker bypass. Roll the in-memory mutation
        //       back to match the (failed) persisted state: atomic fail-closed.
        // ROOT_CAUSE: gate-block persistence shared the lossy save_or_log
        //             path used for cosmetic session fields; disk-full lost
        //             a legitimate trip silently — and partial in-memory
        //             mutation outlived the failed persist.
        // RESEARCH: owasp.org/Top10/2025 A04 Insecure Design (fail-closed);
        //           CWE-392; github.com/akka/akka#26919 (persistence-failure
        //           must not silently weaken a circuit breaker).
        match self.save() {
            Ok(()) => should_trip,
            Err(e) => {
                tracing::warn!(
                    error = %e, category,
                    "kavach-session: gate-block persist failed — failing closed (enforce block, rolling back unpersisted in-memory trip state)"
                );
                if pushed_trip {
                    self.tripped_gate_categories.retain(|c| c != category);
                }
                if let Some(c) = self.gate_block_counts.get_mut(category) {
                    *c = prev_count;
                }
                false
            }
        }
    }

    /// Check if a gate category has tripped (force-allow with advisory).
    /// O(n) where n = number of tripped categories (bounded by ~30).
    #[must_use]
    pub fn is_gate_tripped(&self, category: &str) -> bool {
        self.tripped_gate_categories.iter().any(|c| c == category)
    }

    /// Get block count for a category. O(1) lookup.
    #[must_use]
    pub fn gate_block_count(&self, category: &str) -> i32 {
        self.gate_block_counts
            .get(category)
            .map_or(0, |count| *count)
    }

    // ARCH: ScopeNarrowingHints — acceptance-gated retry with scope narrowing
    // PATTERN: retry_narrowing | SCOPE: session | CAP: AP | SEARCHED: 2026-04
    // Per harness research: on failure, narrow scope instead of retrying same params.
    // Block 1: full error. Block 2: suggest narrowing. Block 3: circuit trips.

    /// Get scope-narrowing hint based on block count for a category.
    /// Returns None if no narrowing needed, Some(hint) if narrowing suggested.
    #[must_use]
    pub fn scope_narrowing_hint(&self, category: &str) -> Option<String> {
        let count = self.gate_block_count(category);
        if count == 0 || count == 1 {
            // First block: just show error, no narrowing hint yet
            None
        } else if count == 2 {
            // Second block: suggest narrowing scope
            Some(format!(
                "[SCOPE_NARROW] Block {count}/3 for {category}. \
                 NARROW YOUR SCOPE: Focus on ONE file at a time. \
                 Complete that file fully before moving to the next. \
                 Broad multi-file changes compound errors."
            ))
        } else {
            // Third+ block: circuit tripped or will trip
            Some(format!(
                "[CIRCUIT_TRIPPED] {category} blocked {count} times. \
                 Force-allowing with advisory. Review tripped categories \
                 at session end."
            ))
        }
    }

    /// Check if we should suggest scope narrowing (block count == 2).
    #[must_use]
    pub fn should_suggest_narrowing(&self, category: &str) -> bool {
        self.gate_block_count(category) == 2
    }

    // ARCH: AutonomousLoopControl — harness engineering loop-until-complete
    // PATTERN: pev_loop | SCOPE: session | CAP: AP | SEARCHED: 2026-05
    // SOURCE: martinfowler.com/articles/harness-engineering.html

    /// Start an autonomous loop with a target condition.
    /// Target formats: "phase:TEST", "kanban:empty", "goal".
    pub fn start_loop(&mut self, target: &str) {
        self.loop_active = true;
        self.loop_target = target.into();
        self.loop_iteration = 0;
        self.loop_start_turn = self.turn_count;
        self.save_or_log();
    }

    /// Increment loop iteration counter. Called by stop gate on each continuation.
    pub fn increment_loop(&mut self) {
        self.loop_iteration = self.loop_iteration.saturating_add(1);
        self.save_or_log();
    }

    /// Check if loop has exceeded max iterations (safety valve).
    #[must_use]
    pub const fn loop_exceeded_max(&self) -> bool {
        self.loop_active && self.loop_iteration >= self.loop_max_iterations
    }

    /// Check if loop target has been reached.
    #[must_use]
    pub fn loop_target_reached(&self) -> bool {
        if !self.loop_active {
            return true;
        }
        match self.loop_target.as_str() {
            "kanban:empty" => matches!(self.loop_kanban_runnable, Some(0)),
            // Proof-gated: a goal is "reached" only when a verified oracle
            // receipt set `goal_receipt_pass`. `goal_achieved` is a legacy
            // fallback for oracle-less goals — a self-asserted flag, kept solely
            // for backward compatibility. roadmap.unit.goal-oracle-workflow.
            "goal" => self.goal_receipt_pass || self.goal_achieved,
            t if t.starts_with("phase:") => {
                let target_phase = t.strip_prefix("phase:").unwrap_or("");
                self.current_phase == target_phase
            }
            _ => false,
        }
    }

    /// Stop the autonomous loop.
    pub fn stop_loop(&mut self) {
        self.loop_active = false;
        self.save_or_log();
    }

    /// Accumulate token spend for the current loop. Called by the stop gate on
    /// each continuation with the turn's token usage. Saturating so a runaway
    /// count can never wrap and silently re-open a closed budget.
    pub fn record_token_spend(&mut self, tokens: i32) {
        self.token_budget_used = self.token_budget_used.saturating_add(tokens.max(0));
        self.save_or_log();
    }

    /// Token-budget safety valve (sibling of `loop_exceeded_max`): true once
    /// cumulative spend reaches the budget. A non-positive `token_budget_total`
    /// means "unbounded" (budget disabled) — never trips, so existing sessions
    /// that never set a budget are unaffected. Prevents runaway agent cost
    /// (`ByteByteGo` harness anatomy: bound the loop by spend, not just steps).
    #[must_use]
    pub const fn budget_exceeded(&self) -> bool {
        self.token_budget_total > 0 && self.token_budget_used >= self.token_budget_total
    }

    /// Check if loop should continue (active, within iteration cap, within
    /// token budget, target not reached). Budget is a hard halt: a runaway
    /// loop must stop on cost even if the target is unmet.
    #[must_use]
    pub fn should_continue_loop(&self) -> bool {
        self.loop_active
            && !self.loop_exceeded_max()
            && !self.budget_exceeded()
            && !self.loop_target_reached()
    }
}

#[cfg(test)]
mod tests {
    use crate::state::SessionState;

    #[test]
    fn test_skill_enforcement() {
        let mut s = SessionState::default();
        s.required_skills = vec!["rust".into(), "security".into()];
        assert!(!s.all_skills_satisfied());
        assert_eq!(s.missing_skills(), vec!["rust", "security"]);

        s.invoked_skills.push("rust".into());
        assert!(!s.all_skills_satisfied());
        assert_eq!(s.missing_skills(), vec!["security"]);

        s.invoked_skills.push("security".into());
        assert!(s.all_skills_satisfied());
        assert!(s.missing_skills().is_empty());
    }

    #[test]
    fn gate_block_trips_at_threshold_on_successful_persist() {
        // No-regression guard for the fail-closed fix: on the normal
        // (save Ok) path, record_gate_block must still return the correct
        // trip decision. Fail-closed (save Err -> false) is fault-injection
        // tested separately (needs an unwritable state dir harness).
        let mut s = SessionState::default();
        s.gate_circuit_breaker_threshold = 3;
        assert!(!s.record_gate_block("deferral"), "block 1 must not trip");
        assert!(!s.record_gate_block("deferral"), "block 2 must not trip");
        assert!(s.record_gate_block("deferral"), "block 3 must trip");
        assert!(s.is_gate_tripped("deferral"));
        // A different category is independent.
        assert!(
            !s.record_gate_block("permission"),
            "other category independent"
        );
    }

    /// Fault-injection: when the state dir is unwritable, `record_gate_block`
    /// must FAIL CLOSED — return `false` (do not report a circuit trip) even
    /// when the in-memory count reaches the threshold, because a reported trip
    /// force-allows in the behavioral gate. A lost persist must keep blocking,
    /// never silently weaken the circuit breaker (CWE-392 / OWASP A04).
    ///
    /// Harness: redirect `state_dir()` (thread-local test seam — no `unsafe`
    /// env mutation, parallel-safe) to a tmpdir, then chmod it `r-x` so
    /// `save()`'s `fs::write` fails with EACCES. A Drop guard restores write
    /// perms, removes the tree, and clears the seam so the test is hermetic
    /// even if an assertion panics.
    #[cfg(unix)]
    #[test]
    fn record_gate_block_fails_closed_when_state_dir_unwritable() {
        use std::os::unix::fs::PermissionsExt;

        struct Restore {
            tmp: std::path::PathBuf,
        }
        impl Drop for Restore {
            #[expect(
                clippy::print_stderr,
                reason = "test-only cleanup guard: stderr is the only diagnostic channel in a Drop \
                          with no tracing subscriber; a leaked perms/dir failure must stay visible"
            )]
            fn drop(&mut self) {
                if let Ok(m) = std::fs::metadata(&self.tmp) {
                    let mut p = m.permissions();
                    p.set_mode(0o700);
                    if let Err(e) = std::fs::set_permissions(&self.tmp, p) {
                        eprintln!("test cleanup: restore perms failed: {e}");
                    }
                }
                if let Err(e) = std::fs::remove_dir_all(&self.tmp) {
                    eprintln!("test cleanup: remove tmp dir failed: {e}");
                }
                crate::paths::set_test_state_dir(None);
            }
        }

        let unique = format!(
            "kavach-faultinj-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        );
        let tmp_dir = std::env::temp_dir().join(unique);

        let _restore = Restore {
            tmp: tmp_dir.clone(),
        };

        std::fs::create_dir_all(&tmp_dir).expect("create tmp state dir");
        crate::paths::set_test_state_dir(Some(tmp_dir.clone()));
        let mut ro = std::fs::metadata(&tmp_dir)
            .expect("stat state dir")
            .permissions();
        ro.set_mode(0o500); // r-x------ : reads ok, fs::write inside fails EACCES
        std::fs::set_permissions(&tmp_dir, ro).expect("chmod state dir ro");

        let mut s = SessionState::default();
        s.gate_circuit_breaker_threshold = 1; // first block reaches threshold

        // In-memory count hits threshold, but save() fails → must NOT report a
        // trip. A `true` here would force-allow the gate on disk-full: the
        // exact silent-bypass this fault-injection test guards.
        let tripped = s.record_gate_block("deferral");
        assert!(
            !tripped,
            "record_gate_block must fail closed (return false) when persist fails"
        );
        // Behavioral-gate view must also stay closed: no category reported
        // tripped to the force-allow path despite the in-memory increment.
        assert!(
            !s.is_gate_tripped("deferral"),
            "no tripped category may be reported when the trip could not be persisted"
        );
    }

    #[test]
    fn test_reset_enforcement() {
        let mut s = SessionState::default();
        s.required_skills = vec!["rust".into()];
        s.invoked_skills = vec!["rust".into()];
        s.research_topic = "axum".into();
        s.reset_enforcement();
        assert!(s.required_skills.is_empty());
        assert!(s.invoked_skills.is_empty());
        assert!(s.research_topic.is_empty());
    }

    #[test]
    fn test_record_skill_no_duplicate() {
        let mut s = SessionState::default();
        s.invoked_skills.push("rust".into());
        s.record_skill_invoked("rust");
        assert_eq!(s.invoked_skills.len(), 1);
    }

    #[test]
    fn evidence_window_non_implement_always_passes() {
        let mut s = SessionState::default();
        s.intent_type = "general".into();
        s.websearch_count_since_intent = 0;
        assert!(s.evidence_window_satisfied());

        s.intent_type = "explain".into();
        assert!(s.evidence_window_satisfied());
    }

    #[test]
    fn evidence_window_implement_blocked_without_websearch() {
        let mut s = SessionState::default();
        s.intent_type = "implement".into();
        s.websearch_count_since_intent = 0;
        assert!(!s.evidence_window_satisfied());
    }

    #[test]
    fn evidence_window_implement_passes_after_websearch() {
        let mut s = SessionState::default();
        s.intent_type = "implement".into();
        s.websearch_count_since_intent = 0;
        s.record_websearch();
        assert!(s.evidence_window_satisfied());
    }

    #[test]
    fn record_websearch_increments_counter() {
        let mut s = SessionState::default();
        assert_eq!(s.websearch_count_since_intent, 0);
        s.record_websearch();
        assert_eq!(s.websearch_count_since_intent, 1);
        s.record_websearch();
        assert_eq!(s.websearch_count_since_intent, 2);
    }

    #[test]
    fn record_websearch_saturates_at_i32_max() {
        let mut s = SessionState::default();
        s.websearch_count_since_intent = i32::MAX;
        s.record_websearch();
        assert_eq!(s.websearch_count_since_intent, i32::MAX);
    }

    #[test]
    fn reset_evidence_window_clears_counter_and_anchors_turn() {
        let mut s = SessionState::default();
        s.intent_type = "implement".into();
        s.websearch_count_since_intent = 3;
        s.turn_count = 5;
        s.reset_evidence_window();
        assert_eq!(s.websearch_count_since_intent, 0);
        assert_eq!(s.intent_set_turn, 5);
        assert!(!s.evidence_window_satisfied());
    }

    #[test]
    fn reset_then_research_satisfies_window() {
        let mut s = SessionState::default();
        s.intent_type = "implement".into();
        s.turn_count = 7;
        s.reset_evidence_window();
        assert!(!s.evidence_window_satisfied());
        s.record_websearch();
        assert!(s.evidence_window_satisfied());
    }

    // ARCH: AutonomousLoopTests — coverage for harness loop control
    // PATTERN: pev_loop | SCOPE: session | CAP: AP | SEARCHED: 2026-05

    #[test]
    fn start_loop_sets_active_and_target() {
        let mut s = SessionState::default();
        s.turn_count = 10;
        s.start_loop("kanban:empty");
        assert!(s.loop_active);
        assert_eq!(s.loop_target, "kanban:empty");
        assert_eq!(s.loop_iteration, 0);
        assert_eq!(s.loop_start_turn, 10);
    }

    #[test]
    fn increment_loop_increments_iteration() {
        let mut s = SessionState::default();
        s.start_loop("goal");
        s.increment_loop();
        s.increment_loop();
        assert_eq!(s.loop_iteration, 2);
    }

    #[test]
    fn increment_loop_saturates_at_max() {
        let mut s = SessionState::default();
        s.loop_iteration = i32::MAX;
        s.increment_loop();
        assert_eq!(s.loop_iteration, i32::MAX);
    }

    #[test]
    fn loop_exceeded_max_when_iteration_reaches_limit() {
        let mut s = SessionState::default();
        s.start_loop("goal");
        s.loop_max_iterations = 3;
        s.loop_iteration = 3;
        assert!(s.loop_exceeded_max());
    }

    #[test]
    fn loop_exceeded_max_false_when_inactive() {
        let mut s = SessionState::default();
        s.loop_active = false;
        s.loop_iteration = 100;
        s.loop_max_iterations = 50;
        assert!(!s.loop_exceeded_max());
    }

    #[test]
    fn loop_target_reached_kanban_empty() {
        let mut s = SessionState::default();
        s.start_loop("kanban:empty");
        s.loop_kanban_runnable = Some(0);
        assert!(s.loop_target_reached());
    }

    #[test]
    fn loop_target_reached_kanban_not_empty() {
        let mut s = SessionState::default();
        s.start_loop("kanban:empty");
        s.loop_kanban_runnable = Some(6);
        assert!(!s.loop_target_reached());
    }

    #[test]
    fn loop_target_kanban_uncensused_fails_closed() {
        let mut s = SessionState::default();
        s.start_loop("kanban:empty");
        assert!(!s.loop_target_reached());
    }

    #[test]
    fn loop_target_reached_phase_match() {
        let mut s = SessionState::default();
        s.start_loop("phase:TEST");
        s.current_phase = "TEST".into();
        assert!(s.loop_target_reached());
    }

    #[test]
    fn loop_target_reached_phase_mismatch() {
        let mut s = SessionState::default();
        s.start_loop("phase:TEST");
        s.current_phase = "IMPLEMENT".into();
        assert!(!s.loop_target_reached());
    }

    #[test]
    fn loop_target_reached_goal() {
        let mut s = SessionState::default();
        s.start_loop("goal");
        s.goal_achieved = true;
        assert!(s.loop_target_reached());
    }

    #[test]
    fn loop_target_reached_goal_via_verified_receipt() {
        // The proof-gated path: a verified oracle receipt set goal_receipt_pass.
        let mut s = SessionState::default();
        s.start_loop("goal");
        s.goal_receipt_pass = true;
        assert!(s.loop_target_reached());
    }

    #[test]
    fn loop_target_goal_blocks_without_proof() {
        // THE PHASE-3 ORACLE: an active goal with NO receipt and NO legacy flag
        // is NOT reached — a hallucinated "done" cannot satisfy the stop gate.
        let mut s = SessionState::default();
        s.start_loop("goal");
        assert!(!s.goal_receipt_pass);
        assert!(!s.goal_achieved);
        assert!(!s.loop_target_reached());
    }

    #[test]
    fn goal_receipt_pass_round_trips_through_serde() {
        let mut s = SessionState::default();
        s.start_loop("goal");
        s.goal_receipt_pass = true;
        let ini = s.to_ini_full();
        let parsed = crate::load::parse_ini_str(&ini);
        assert!(
            parsed.goal_receipt_pass,
            "receipt flag must survive persistence"
        );
    }

    #[test]
    fn loop_target_reached_when_inactive() {
        let s = SessionState::default();
        // Inactive loop is "reached" — vacuously true (no work to continue).
        assert!(s.loop_target_reached());
    }

    #[test]
    fn loop_target_reached_unknown_target_returns_false() {
        let mut s = SessionState::default();
        s.start_loop("unknown:target");
        assert!(!s.loop_target_reached());
    }

    #[test]
    fn stop_loop_deactivates() {
        let mut s = SessionState::default();
        s.start_loop("goal");
        s.stop_loop();
        assert!(!s.loop_active);
    }

    #[test]
    fn should_continue_loop_active_target_not_reached() {
        let mut s = SessionState::default();
        s.start_loop("phase:TEST");
        s.current_phase = "IMPLEMENT".into();
        assert!(s.should_continue_loop());
    }

    #[test]
    fn should_continue_loop_false_when_target_reached() {
        let mut s = SessionState::default();
        s.start_loop("phase:TEST");
        s.current_phase = "TEST".into();
        assert!(!s.should_continue_loop());
    }

    #[test]
    fn should_continue_loop_false_when_max_exceeded() {
        let mut s = SessionState::default();
        s.start_loop("goal");
        s.loop_max_iterations = 5;
        s.loop_iteration = 5;
        assert!(!s.should_continue_loop());
    }

    #[test]
    fn should_continue_loop_false_when_inactive() {
        let s = SessionState::default();
        assert!(!s.should_continue_loop());
    }

    // ARCH: TokenBudgetGate — bound the harness loop by spend, not just steps.
    // SOURCE: ByteByteGo agent-harness anatomy (runaway-cost prevention).

    #[test]
    fn budget_not_exceeded_when_total_unset() {
        let mut s = SessionState::default();
        s.token_budget_total = 0; // unbounded / disabled
        s.token_budget_used = 999_999;
        assert!(!s.budget_exceeded(), "zero total = unbounded, never trips");
    }

    #[test]
    fn budget_not_exceeded_below_total() {
        let mut s = SessionState::default();
        s.token_budget_total = 1000;
        s.token_budget_used = 999;
        assert!(!s.budget_exceeded());
    }

    #[test]
    fn budget_exceeded_at_and_above_total() {
        let mut s = SessionState::default();
        s.token_budget_total = 1000;
        s.token_budget_used = 1000;
        assert!(s.budget_exceeded(), "spend == total trips (>=)");
        s.token_budget_used = 1500;
        assert!(s.budget_exceeded());
    }

    #[test]
    fn record_token_spend_accumulates_and_clamps_negative() {
        let mut s = SessionState::default();
        s.record_token_spend(100);
        s.record_token_spend(50);
        assert_eq!(s.token_budget_used, 150);
        s.record_token_spend(-999); // negative ignored, never decrements
        assert_eq!(s.token_budget_used, 150);
    }

    #[test]
    fn record_token_spend_saturates_at_i32_max() {
        let mut s = SessionState::default();
        s.token_budget_used = i32::MAX;
        s.record_token_spend(1000);
        assert_eq!(s.token_budget_used, i32::MAX, "no wrap re-opening budget");
    }

    #[test]
    fn should_continue_loop_false_when_budget_exceeded() {
        let mut s = SessionState::default();
        s.start_loop("goal"); // active, target not reached
        s.token_budget_total = 1000;
        s.token_budget_used = 1000;
        assert!(
            !s.should_continue_loop(),
            "budget halt overrides an unmet target"
        );
    }
}

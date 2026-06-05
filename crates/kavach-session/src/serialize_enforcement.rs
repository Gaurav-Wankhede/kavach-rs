use std::fmt::Write as FmtWrite;

use crate::save::join_csv;
use crate::serialize::write_kv;
use crate::state::SessionState;

impl SessionState {
    /// Serialize enforcement, test, loop guard, and team tracking sections.
    pub(crate) fn serialize_enforcement_sections(&self, s: &mut String) {
        if !self.required_skills.is_empty()
            || !self.invoked_skills.is_empty()
            || !self.research_topic.is_empty()
        {
            s.push_str("[ENFORCEMENT]\n");
            // Always write required_skills — even when empty — so a cleared list
            // overwrites any stale value from a prior turn in the INI file.
            write_kv(s, "required_skills", &join_csv(&self.required_skills));
            if !self.invoked_skills.is_empty() {
                write_kv(s, "invoked_skills", &join_csv(&self.invoked_skills));
            }
            if !self.research_topic.is_empty() {
                write_kv(s, "research_topic", &self.research_topic);
            }
            s.push('\n');
        }

        if !self.recent_commands.is_empty() {
            s.push_str("[LOOP_GUARD]\n");
            write_kv(s, "recent_commands", &join_csv(&self.recent_commands));
            s.push('\n');
        }

        // FIX [state_drift / lost_update] — test_nudge_count is a cross-turn
        // counter; previously nested under [TEST_ENFORCEMENT] gated on
        // test_files_pending non-empty, so clearing the list while count > 0
        // dropped the counter. Same class as stop_reblock_count and
        // gate_block_counts. Persist in its own unconditional section.
        if !self.test_files_pending.is_empty() {
            s.push_str("[TEST_ENFORCEMENT]\n");
            write_kv(s, "test_files_pending", &join_csv(&self.test_files_pending));
            s.push('\n');
        }
        if self.test_nudge_count != 0 {
            s.push_str("[TEST_NUDGE]\n");
            write_kv(s, "test_nudge_count", &self.test_nudge_count.to_string());
            s.push('\n');
        }

        // FIX [state_drift] — intent_risk previously skipped emission when
        // value was the literal "low", treating "low" as absence-marker. A
        // transition FROM "high" TO "low" was never persisted; next load saw
        // stale "high". SOURCE: github.com/Kotlin/kotlinx.serialization#2586.
        // Persist whenever non-empty; "low" is a legitimate explicit value.
        if !self.intent_risk.is_empty() {
            s.push_str("[INTENT_RISK]\n");
            write_kv(s, "intent_risk", &self.intent_risk);
            s.push('\n');
        }

        if self.subagent_files_read > 0 {
            s.push_str("[ATTENTION_TRACKING]\n");
            write_kv(
                s,
                "subagent_files_read",
                &self.subagent_files_read.to_string(),
            );
            s.push('\n');
        }

        if !self.case_facts.is_empty() {
            s.push_str("[CASE_FACTS]\n");
            for fact in &self.case_facts {
                writeln!(s, "- {fact}").ok();
            }
            s.push('\n');
        }

        if !self.team_name.is_empty() || !self.team_members.is_empty() {
            s.push_str("[TEAM]\n");
            write_kv(s, "team_name", &self.team_name);
            if !self.team_members.is_empty() {
                write_kv(s, "team_members", &join_csv(&self.team_members));
            }
            write_kv(s, "active_teammates", &self.active_teammates.to_string());
            write_kv(s, "model_id", &self.model_id);
            s.push('\n');
        }
    }
}

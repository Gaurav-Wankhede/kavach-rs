use std::fmt::Write as _;

use crate::save::join_csv;
use crate::state::SessionState;

pub(crate) fn write_kv(s: &mut String, key: &str, value: &str) {
    writeln!(s, "{key}: {value}").ok();
}

pub(crate) fn bool_str(b: bool) -> String {
    if b { "true" } else { "false" }.into()
}

pub(crate) fn write_files_array(s: &mut String, files: &[String]) {
    if files.is_empty() {
        s.push_str("files[]:\n");
    } else if let [single] = files {
        writeln!(s, "files[]: {single}").ok();
    } else {
        s.push_str("files[]:\n");
        for f in files {
            writeln!(s, "  - {f}").ok();
        }
    }
}

impl SessionState {
    #[must_use]
    pub fn to_ini_full(&self) -> String {
        let mut s = String::with_capacity(2048);
        s.push_str("# Session State - SP/3.0\n");
        s.push_str("# Auto-generated, do not edit\n\n");
        self.serialize_core(&mut s);
        self.serialize_extras(&mut s);
        s
    }

    fn serialize_core(&self, s: &mut String) {
        s.push_str("[SESSION]\n");
        write_kv(s, "id", &self.id);
        write_kv(s, "today", &self.today);
        write_kv(s, "project", &self.project);
        write_kv(s, "workdir", &self.work_dir);
        write_kv(s, "cutoff", &self.training_cutoff);
        s.push('\n');

        s.push_str("[STATE]\n");
        write_kv(s, "research_done", &bool_str(self.research_done));
        if !self.research_topics.is_empty() {
            write_kv(s, "research_topics", &join_csv(&self.research_topics));
        }
        write_kv(s, "memory", &bool_str(self.memory_queried));
        write_kv(s, "turn_count", &self.turn_count.to_string());
        write_kv(
            s,
            "last_reinforce_turn",
            &self.last_reinforce_turn.to_string(),
        );
        write_kv(s, "reinforce_every_n", &self.reinforce_every_n.to_string());
        write_kv(s, "tasks_created", &self.tasks_created.to_string());
        write_kv(s, "tasks_completed", &self.tasks_completed.to_string());
        write_kv(s, "session_id", &self.session_id);
        s.push('\n');

        s.push_str("[COMPACT]\n");
        write_kv(s, "post_compact", &bool_str(self.post_compact));
        write_kv(s, "compacted_at", &self.compacted_at);
        write_kv(s, "compact_count", &self.compact_count.to_string());
        s.push('\n');

        s.push_str("[TASK]\n");
        write_kv(s, "task", &self.current_task);
        write_kv(s, "task_status", &self.task_status);
        if !self.task_list_id.is_empty() {
            write_kv(s, "task_list_id", &self.task_list_id);
        }
        write_files_array(s, &self.files_modified);
        if self.last_write_turn > 0 {
            write_kv(s, "last_write_turn", &self.last_write_turn.to_string());
        }
        if self.last_db_write_turn > 0 {
            write_kv(
                s,
                "last_db_write_turn",
                &self.last_db_write_turn.to_string(),
            );
        }
        s.push('\n');
    }
}

use crate::paths::{detect_project, today};
use crate::state::SessionState;
use chrono::Local;
mod circuit_breaker;
mod loop_control;
mod research;
mod skills;
mod test_tracking;
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
}
#[cfg(test)]
#[path = "enforcement/enforcement_test.rs"]
mod tests;

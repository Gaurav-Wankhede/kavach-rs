use crate::cmd::io_safe::{ewrite_or_exit, into_exit_code, print_or_exit};

pub(crate) fn handle_spike_start(project: &str, hours: u32, reason: &str) -> i32 {
    if hours == 0 {
        if let Err(io_err) = ewrite_or_exit("hours must be > 0") {
            return into_exit_code(io_err);
        }
        return 1;
    }
    let now = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => i64::try_from(d.as_secs()).map_or(i64::MAX, |v| v),
        Err(e) => {
            let msg = format!("system clock before UNIX_EPOCH: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let expires_at = now.saturating_add(i64::from(hours).saturating_mul(3600));
    let content =
        format!("started_at_unix_s={now}\nexpires_at_unix_s={expires_at}\nreason={reason}");

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("tokio: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let result: Result<(), String> = runtime.block_on(async {
        let db = kavach_surreal::open_default_resilient()
            .await
            .map_err(|e| format!("open db: {e}"))?;
        let project_rec = kavach_surreal::projects::get_by_slug(&db, project)
            .await
            .map_err(|e| format!("get project: {e}"))?
            .ok_or_else(|| format!("project not found: {project}"))?;
        let pid = project_rec
            .id
            .ok_or_else(|| "project missing id".to_owned())?;
        kavach_surreal::write::upsert_entry_full()
            .db(&db)
            .category("decision")
            .project_id(&pid)
            .entry_key("workflow.spike.active")
            .title("Spike mode active")
            .content(&content)
            .event_source("phase spike-start")
            .qualified_name("")
            .references(&[])
            .build_for_call()
            .await
            .map_err(|e| format!("write spike row: {e}"))?;
        Ok(())
    });
    if let Err(e) = result {
        let msg = format!("spike-start failed: {e}");
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }
    let ok = format!(
        "[SPIKE_START] project={project} hours={hours} expires_at_unix_s={expires_at} reason={reason}",
    );
    if let Err(io_err) = print_or_exit(&ok) {
        return into_exit_code(io_err);
    }
    0
}

pub(crate) fn handle_spike_end(project: &str) -> i32 {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("tokio: {e}");
            if let Err(io_err) = ewrite_or_exit(&msg) {
                return into_exit_code(io_err);
            }
            return 1;
        }
    };
    let result: Result<(), String> = runtime.block_on(async {
        let db = kavach_surreal::open_default_resilient()
            .await
            .map_err(|e| format!("open db: {e}"))?;
        let project_rec = kavach_surreal::projects::get_by_slug(&db, project)
            .await
            .map_err(|e| format!("get project: {e}"))?
            .ok_or_else(|| format!("project not found: {project}"))?;
        let pid = project_rec
            .id
            .ok_or_else(|| "project missing id".to_owned())?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_secs()).map_or(i64::MAX, |v| v));
        let content = format!("expires_at_unix_s={now}\nreason=spike-ended");
        kavach_surreal::write::upsert_entry_full()
            .db(&db)
            .category("decision")
            .project_id(&pid)
            .entry_key("workflow.spike.active")
            .title("Spike mode ended")
            .content(&content)
            .event_source("phase spike-end")
            .qualified_name("")
            .references(&[])
            .build_for_call()
            .await
            .map_err(|e| format!("write spike-end: {e}"))?;
        Ok(())
    });
    if let Err(e) = result {
        let msg = format!("spike-end failed: {e}");
        if let Err(io_err) = ewrite_or_exit(&msg) {
            return into_exit_code(io_err);
        }
        return 1;
    }
    let ok = format!("[SPIKE_END] project={project}");
    if let Err(io_err) = print_or_exit(&ok) {
        return into_exit_code(io_err);
    }
    0
}

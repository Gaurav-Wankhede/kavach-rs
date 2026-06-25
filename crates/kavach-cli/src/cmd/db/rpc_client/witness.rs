pub(crate) fn mint_receipt() -> Option<kavach_patterns::witness_receipt::Receipt> {
    let head = git_head()?;
    let session_id = {
        let s = kavach_session::get_or_create_session().session_id;
        if s.is_empty() { "cli".to_owned() } else { s }
    };
    let ts_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX));
    Some(kavach_patterns::witness_receipt::Receipt::new(
        true, head, ts_ms, session_id,
    ))
}

fn git_head() -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let t = s.trim();
    (!t.is_empty()).then(|| t.to_owned())
}

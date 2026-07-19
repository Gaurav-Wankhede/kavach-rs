pub(crate) fn mint_receipt() -> Option<kavach_patterns::witness_receipt::Receipt> {
    let head = git_head();
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

fn git_head() -> String {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output();
    match out {
        Ok(out) if out.status.success() => {
            String::from_utf8(out.stdout).map_or(String::new(), |s| {
                let t = s.trim();
                if t.is_empty() { String::new() } else { t.to_owned() }
            })
        }
        _ => String::new(),
    }
}

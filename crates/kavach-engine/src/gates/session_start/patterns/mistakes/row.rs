//! Mistake-ledger row fetch + token parsing (`hit_count` / `last_seen_unix`).

pub(super) struct MistakeRow {
    pub(super) title: String,
    pub(super) hit_count: u32,
    pub(super) last_seen_unix: u64,
}

/// Fetch a single row's full body and parse `hit_count=` / `last_seen_unix=`.
/// Falls back to count=1, `last_seen=now` for rows lacking the tokens (older
/// rows recorded before the K-PRI schema landed).
pub(super) fn fetch_mistake_row(project: &str, key: &str) -> Option<MistakeRow> {
    let out = std::process::Command::new("kavach")
        .args([
            "db",
            "get",
            "--project",
            project,
            "--category",
            "pattern",
            "--key",
            key,
            "--full",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let body = String::from_utf8(out.stdout).ok()?;
    // SECURITY: extract title from the structured metadata header ONLY.
    // The previous OR-fallback `l.contains("BANNED [")` was a false-positive
    // sink: row CONTENT echoes the raw banned_sample verbatim (intentionally,
    // for the learning diff trail), and a banned_sample like "should I proceed
    // BANNED [permission]: ..." would let the fallback pick the content body
    // and surface the raw banned phrase to the model — defeating the arxiv
    // 2512.02389 anti-pattern-reinjection framing. Title-prefix is the only
    // safe extraction path; mistake_ledger.rs always sets a title, so the
    // fallback is dead-but-unsafe code.
    let title = body.lines().find(|l| l.starts_with("title:")).map_or_else(
        || key.to_owned(),
        |l| l.trim_start_matches("title:").trim().to_owned(),
    );
    let hit_count =
        u32::try_from(parse_int_token(&body, "hit_count=").unwrap_or(1)).unwrap_or(u32::MAX);
    let last_seen_unix = parse_int_token(&body, "last_seen_unix=").unwrap_or(0);
    Some(MistakeRow {
        title,
        hit_count,
        last_seen_unix,
    })
}

fn parse_int_token(body: &str, marker: &str) -> Option<u64> {
    let idx = body.find(marker)?;
    let tail = body.get(idx.saturating_add(marker.len())..)?;
    let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
    digits.parse::<u64>().ok()
}

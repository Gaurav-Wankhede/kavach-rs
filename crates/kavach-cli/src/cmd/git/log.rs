use serde_json::json;

pub(crate) fn run(limit: usize, oneline: bool) -> i32 {
    let repo = match gix::open(".") {
        Ok(r) => r,
        Err(e) => {
            eprintln!("kavach git: not in a git repository: {}", e);
            return 2;
        }
    };

    let head = match repo.head_commit() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("kavach git: failed to get HEAD commit: {}", e);
            return 1;
        }
    };

    let commits = match head.ancestors().all() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("kavach git: failed to traverse commit history: {}", e);
            return 1;
        }
    };

    let mut records = Vec::new();

    for commit_info_res in commits.take(limit) {
        let commit_info = match commit_info_res {
            Ok(c) => c,
            Err(_) => continue,
        };

        let commit = match commit_info.object() {
            Ok(c) => c,
            Err(_) => continue,
        };

        let short_id = &commit.id.to_hex().to_string()[..8];
        let author_ref = commit.author().ok();
        let author_name = author_ref.as_ref().map(|a| a.name.to_string()).unwrap_or_else(|| "Unknown".to_string());
        let date_str = author_ref.as_ref().map(|a| {
            let secs = a.time().map(|t| t.seconds).unwrap_or_default();
            chrono::DateTime::from_timestamp(secs, 0)
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_default()
        }).unwrap_or_default();
        let summary = commit.message().ok().map(|m| m.summary().to_string()).unwrap_or_default();

        if oneline {
            println!("{} {}", short_id, summary);
        } else {
            records.push(json!({
                "commit": short_id,
                "author": author_name,
                "date": date_str,
                "message": summary,
            }));
        }
    }

    if !oneline && !records.is_empty() {
        let v = json!(records);
        let toon = kavach_toon::encode_toon(&v);
        println!("{}", toon.trim_end());
    }

    0
}

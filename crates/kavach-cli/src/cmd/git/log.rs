use git2::Repository;
use serde_json::json;

pub(crate) fn run(limit: usize, oneline: bool) -> i32 {
    let repo = match Repository::discover(".") {
        Ok(r) => r,
        Err(e) => {
            eprintln!("kavach git: not in a git repository: {}", e);
            return 2;
        }
    };

    let mut revwalk = match repo.revwalk() {
        Ok(rw) => rw,
        Err(e) => {
            eprintln!("kavach git: failed to traverse revisions: {}", e);
            return 1;
        }
    };

    if let Err(e) = revwalk.push_head() {
        eprintln!("kavach git: failed to read HEAD: {}", e);
        return 1;
    }

    let mut commits = Vec::new();

    for oid_res in revwalk.take(limit) {
        let oid = match oid_res {
            Ok(id) => id,
            Err(_) => continue,
        };

        if let Ok(commit) = repo.find_commit(oid) {
            let short_id = format!("{:.8}", commit.id());
            let summary = commit.summary().unwrap_or(Some("")).unwrap_or("").to_string();
            let author = commit.author().name().unwrap_or("unknown").to_string();
            let time = commit.time().seconds();
            let naive = chrono::DateTime::from_timestamp(time, 0)
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_default();

            if oneline {
                println!("{} {}", short_id, summary);
            } else {
                commits.push(json!({
                    "commit": short_id,
                    "date": naive,
                    "author": author,
                    "message": summary,
                }));
            }
        }
    }

    if !oneline && !commits.is_empty() {
        let v = json!(commits);
        let toon = kavach_toon::encode_toon(&v);
        println!("{}", toon.trim_end());
    }

    0
}

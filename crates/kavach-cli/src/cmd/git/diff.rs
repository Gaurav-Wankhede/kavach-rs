use git2::Repository;
use serde_json::json;

pub(crate) fn run(stat_only: bool, files_only: bool) -> i32 {
    let repo = match Repository::discover(".") {
        Ok(r) => r,
        Err(e) => {
            eprintln!("kavach git: not in a git repository: {}", e);
            return 2;
        }
    };

    let head = repo.head().and_then(|h| h.peel_to_tree()).ok();
    let diff = match repo.diff_tree_to_workdir_with_index(head.as_ref(), None) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("kavach git: failed to compute diff: {}", e);
            return 1;
        }
    };

    let stats = match diff.stats() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("kavach git: failed to compute diff stats: {}", e);
            return 1;
        }
    };

    if files_only {
        let mut files = Vec::new();
        for delta in diff.deltas() {
            if let Some(path) = delta.new_file().path().and_then(|p| p.to_str()) {
                files.push(path.to_string());
            }
        }
        for f in files {
            println!("{}", f);
        }
        return 0;
    }

    if stat_only {
        println!("{} files changed, {} insertions(+), {} deletions(-)",
            stats.files_changed(),
            stats.insertions(),
            stats.deletions()
        );
        return 0;
    }

    // Default compact diff summary in TOON
    let mut deltas = Vec::new();
    for delta in diff.deltas() {
        let status = format!("{:?}", delta.status());
        let path = delta.new_file().path().and_then(|p| p.to_str()).unwrap_or("").to_string();
        deltas.push(json!({
            "status": status,
            "path": path,
        }));
    }

    if !deltas.is_empty() {
        let v = json!(deltas);
        let toon = kavach_toon::encode_toon(&v);
        println!("{}", toon.trim_end());
    }
    println!("── Summary: {} files changed, +{}, -{} ──", stats.files_changed(), stats.insertions(), stats.deletions());

    0
}

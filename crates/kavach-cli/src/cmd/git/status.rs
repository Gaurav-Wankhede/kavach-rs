use git2::Repository;
use serde_json::json;

pub(crate) fn run() -> i32 {
    let repo = match Repository::discover(".") {
        Ok(r) => r,
        Err(e) => {
            eprintln!("kavach git: not in a git repository: {}", e);
            return 2;
        }
    };

    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(false);

    let statuses = match repo.statuses(Some(&mut opts)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("kavach git: failed to get statuses: {}", e);
            return 1;
        }
    };

    if statuses.is_empty() {
        println!("working tree clean");
        return 0;
    }

    let mut entries = Vec::new();
    for entry in statuses.iter() {
        let path = entry.path().unwrap_or("").to_string();
        let status_bits = entry.status();
        let status_str = if status_bits.is_wt_new() {
            "?"
        } else if status_bits.is_wt_modified() || status_bits.is_index_modified() {
            "M"
        } else if status_bits.is_wt_deleted() || status_bits.is_index_deleted() {
            "D"
        } else if status_bits.is_wt_renamed() || status_bits.is_index_renamed() {
            "R"
        } else {
            "*"
        };

        entries.push(json!({
            "status": status_str,
            "path": path,
        }));
    }

    let v = json!(entries);
    let toon = kavach_toon::encode_toon(&v);
    println!("{}", toon.trim_end());

    0
}

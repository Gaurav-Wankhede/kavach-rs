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

    let head_name = repo.head().ok().and_then(|h| h.shorthand().ok().map(|s| s.to_string())).unwrap_or_else(|| "HEAD".to_string());

    let branches = match repo.branches(Some(git2::BranchType::Local)) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("kavach git: failed to list branches: {}", e);
            return 1;
        }
    };

    let mut branch_records = Vec::new();
    for b in branches {
        if let Ok((branch, _)) = b {
            if let Ok(Some(name)) = branch.name() {
                let is_current = name == head_name;
                branch_records.push(json!({
                    "current": if is_current { "*" } else { " " },
                    "name": name,
                }));
            }
        }
    }

    let v = json!(branch_records);
    let toon = kavach_toon::encode_toon(&v);
    println!("{}", toon.trim_end());
    0
}

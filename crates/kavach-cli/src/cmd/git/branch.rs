use serde_json::json;

pub(crate) fn run() -> i32 {
    let repo = match gix::open(".") {
        Ok(r) => r,
        Err(e) => {
            eprintln!("kavach git: not in a git repository: {}", e);
            return 2;
        }
    };

    let head_ref = repo.head_ref().ok().flatten();
    let current_branch_name = head_ref
        .as_ref()
        .and_then(|r| r.name().shorten().to_string().into());

    let platform = match repo.references() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("kavach git: failed to list references: {}", e);
            return 1;
        }
    };

    let local_branches = match platform.local_branches() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("kavach git: failed to iterate local branches: {}", e);
            return 1;
        }
    };

    let mut branch_records = Vec::new();
    for branch_res in local_branches {
        if let Ok(branch) = branch_res {
            let name = branch.name().shorten().to_string();
            let is_current = current_branch_name.as_deref() == Some(&name);
            branch_records.push(json!({
                "current": if is_current { "*" } else { " " },
                "name": name,
            }));
        }
    }

    let v = json!(branch_records);
    let toon = kavach_toon::encode_toon(&v);
    println!("{}", toon.trim_end());
    0
}

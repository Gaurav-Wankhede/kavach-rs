pub(crate) fn run(message: &str, _all: bool) -> i32 {
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
            eprintln!("kavach git: failed to find head commit: {}", e);
            return 1;
        }
    };

    let tree_id = head.tree_id().ok();
    if tree_id.is_none() {
        eprintln!("kavach git: failed to resolve head tree");
        return 1;
    }
    let tree_id = tree_id.unwrap();

    let author_name = repo.author().and_then(|r| r.ok()).map(|s| s.name.to_string()).unwrap_or_else(|| "kavach".to_string());

    let commit_id = match repo.commit(
        "HEAD",
        message,
        tree_id,
        [head.id],
    ) {
        Ok(oid) => oid,
        Err(e) => {
            eprintln!("kavach git: commit failed: {}", e);
            return 1;
        }
    };

    let short_id = &commit_id.to_hex().to_string()[..8];
    println!("commit: {}", short_id);
    println!("\tmessage: {}", message);
    println!("\tauthor: {}", author_name);
    0
}

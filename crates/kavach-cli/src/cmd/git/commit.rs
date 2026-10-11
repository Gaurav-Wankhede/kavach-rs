use git2::{IndexAddOption, Repository, Signature};

pub(crate) fn run(message: &str, all: bool) -> i32 {
    let repo = match Repository::discover(".") {
        Ok(r) => r,
        Err(e) => {
            eprintln!("kavach git: not in a git repository: {}", e);
            return 2;
        }
    };

    let mut index = match repo.index() {
        Ok(i) => i,
        Err(e) => {
            eprintln!("kavach git: failed to open repository index: {}", e);
            return 1;
        }
    };

    if all {
        if let Err(e) = index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None) {
            eprintln!("kavach git: failed to stage files: {}", e);
            return 1;
        }
        if let Err(e) = index.update_all(["*"].iter(), None) {
            eprintln!("kavach git: failed to update staged files: {}", e);
            return 1;
        }
        if let Err(e) = index.write() {
            eprintln!("kavach git: failed to write index: {}", e);
            return 1;
        }
    }

    let tree_id = match index.write_tree() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("kavach git: failed to write tree: {}", e);
            return 1;
        }
    };

    let tree = match repo.find_tree(tree_id) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("kavach git: failed to find tree: {}", e);
            return 1;
        }
    };

    let sig = repo.signature().unwrap_or_else(|_| {
        Signature::now("kavach", "kavach@localhost").expect("signature creation failed")
    });

    let head = repo.head().ok();
    let parent_commit = head.as_ref().and_then(|h| h.peel_to_commit().ok());
    let parents = match &parent_commit {
        Some(c) => vec![c],
        None => vec![],
    };

    let commit_id = match repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        message,
        &tree,
        &parents,
    ) {
        Ok(oid) => oid,
        Err(e) => {
            eprintln!("kavach git: failed to create commit: {}", e);
            return 1;
        }
    };

    let short_id = &commit_id.to_string()[..8];
    println!("commit: {}", short_id);
    println!("\tmessage: {}", message);
    println!("\tauthor: {}", sig.name().unwrap_or("kavach"));
    0
}

pub(crate) fn run(target: &str, def_symbol: Option<&str>, call_symbol: Option<&str>) -> i32 {
    let repo = match gix::open(".") {
        Ok(r) => r,
        Err(e) => {
            eprintln!("kavach git: not in a git repository: {}", e);
            return 2;
        }
    };

    let (rev, file_path) = match target.split_once(':') {
        Some((r, p)) => (r, p),
        None => {
            eprintln!("kavach git show: expected <rev>:<path> (e.g. HEAD:src/lib.rs)");
            return 2;
        }
    };

    let spec = match repo.rev_parse_single(rev) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("kavach git show: revision '{}' not found: {}", rev, e);
            return 1;
        }
    };

    let object = match spec.object() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("kavach git show: object not found for revision '{}': {}", rev, e);
            return 1;
        }
    };

    let commit = match object.try_into_commit() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("kavach git show: revision '{}' is not a commit: {}", rev, e);
            return 1;
        }
    };

    let tree = match commit.tree() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("kavach git show: failed to get commit tree: {}", e);
            return 1;
        }
    };

    let entry = match tree.lookup_entry_by_path(file_path) {
        Ok(Some(e)) => e,
        Ok(None) => {
            eprintln!("kavach git show: file '{}' not found in revision '{}'", file_path, rev);
            return 1;
        }
        Err(e) => {
            eprintln!("kavach git show: error looking up '{}': {}", file_path, e);
            return 1;
        }
    };

    let object = match entry.object() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("kavach git show: cannot read blob '{}': {}", file_path, e);
            return 1;
        }
    };

    let content = match std::str::from_utf8(&object.data) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("kavach git show: '{}' is a binary file", file_path);
            return 1;
        }
    };

    if let Some(def) = def_symbol {
        let mut found = false;
        let mut lines_iter = content.lines().enumerate();
        while let Some((idx, line)) = lines_iter.next() {
            if line.contains(&format!("fn {}", def))
                || line.contains(&format!("struct {}", def))
                || line.contains(&format!("enum {}", def))
                || line.contains(&format!("pub fn {}", def))
                || line.contains(&format!("pub(crate) fn {}", def))
            {
                found = true;
                println!("{}:{}:{}", file_path, idx + 1, line);
                let mut brace_depth = line.matches('{').count() as i32 - line.matches('}').count() as i32;
                while brace_depth > 0 {
                    if let Some((sub_idx, sub_line)) = lines_iter.next() {
                        brace_depth += sub_line.matches('{').count() as i32 - sub_line.matches('}').count() as i32;
                        println!("{}:{}:{}", file_path, sub_idx + 1, sub_line);
                    } else {
                        break;
                    }
                }
                break;
            }
        }
        if !found {
            eprintln!("kavach git show: definition '{}' not found in {} @ {}", def, file_path, rev);
            return 1;
        }
        return 0;
    }

    if let Some(call) = call_symbol {
        let pattern = format!("{}(", call);
        for (idx, line) in content.lines().enumerate() {
            if line.contains(&pattern) {
                println!("{}:{}:{}", file_path, idx + 1, line.trim());
            }
        }
        return 0;
    }

    let line_count = content.lines().count();
    if line_count > 60 {
        eprintln!(
            "kavach git show: file has {} lines. Whole-file dump denied to preserve context.\n                 Specify an AST target:\n                     kavach git show {} --def <symbol>\n                     kavach git show {} --call <symbol>",
            line_count, target, target
        );
        return 2;
    }

    print!("{}", content);
    0
}

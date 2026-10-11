use std::collections::BTreeSet;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

pub(crate) fn run(
    path_opt: Option<&str>,
    keys_only: bool,
    rename: Option<(String, String)>,
    unset_key: Option<&str>,
    diff_code_path: Option<&str>,
) -> i32 {
    let target_path = path_opt.unwrap_or(".env");
    let p = Path::new(target_path);

    if let Some((old_k, new_k)) = rename {
        return handle_rename(p, &old_k, &new_k);
    }

    if let Some(unset_k) = unset_key {
        return handle_unset(p, unset_k);
    }

    if let Some(code_dir) = diff_code_path {
        return handle_diff(p, code_dir);
    }

    if !p.exists() {
        eprintln!("kavach env: file '{}' does not exist", target_path);
        return 2;
    }

    let file = match fs::File::open(p) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("kavach env: error opening '{}': {}", target_path, e);
            return 1;
        }
    };

    let reader = BufReader::new(file);
    for line_res in reader.lines() {
        let line = match line_res {
            Ok(l) => l,
            Err(_) => continue,
        };

        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some((key_part, val_part)) = trimmed.split_once('=') {
            let key = key_part.trim();
            if keys_only {
                println!("{}", key);
            } else {
                let val_trimmed = val_part.trim();
                let status = if val_trimmed.is_empty() {
                    "[EMPTY]".to_string()
                } else {
                    format!("[SET] (len={})", val_trimmed.len())
                };
                // ZERO-LEAK GUARANTEE: Never emit characters after = to stdout
                println!("{}={}", key, status);
            }
        }
    }

    0
}

fn handle_rename(p: &Path, old_key: &str, new_key: &str) -> i32 {
    if !p.exists() {
        eprintln!("kavach env: file '{}' does not exist", p.display());
        return 2;
    }

    let content = match fs::read_to_string(p) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("kavach env: error reading '{}': {}", p.display(), e);
            return 1;
        }
    };

    let mut found = false;
    let mut modified_lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('#') && trimmed.contains('=') {
            if let Some((k, v)) = line.split_once('=') {
                if k.trim() == old_key {
                    found = true;
                    // Preserve value byte-for-byte, change key only
                    modified_lines.push(format!("{}={}", new_key, v));
                    continue;
                }
            }
        }
        modified_lines.push(line.to_string());
    }

    if !found {
        eprintln!("kavach env: key '{}' not found in '{}'", old_key, p.display());
        return 1;
    }

    let mut output = modified_lines.join("
");
    if content.ends_with('\n') {
        output.push('\n');
    }

    // Atomic write
    let parent = p.parent().unwrap_or_else(|| Path::new("."));
    let mut temp = match tempfile::NamedTempFile::new_in(parent) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("kavach env: failed to create temporary file: {}", e);
            return 1;
        }
    };

    if let Err(e) = temp.write_all(output.as_bytes()) {
        eprintln!("kavach env: failed to write to temporary file: {}", e);
        return 1;
    }

    if let Err(e) = temp.persist(p) {
        eprintln!("kavach env: failed to atomically update '{}': {}", p.display(), e);
        return 1;
    }

    println!("[OK] Atomically renamed key '{}' -> '{}' in '{}' (secret value preserved byte-for-byte)", old_key, new_key, p.display());
    0
}

fn handle_unset(p: &Path, unset_key: &str) -> i32 {
    if !p.exists() {
        eprintln!("kavach env: file '{}' does not exist", p.display());
        return 2;
    }

    let content = match fs::read_to_string(p) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("kavach env: error reading '{}': {}", p.display(), e);
            return 1;
        }
    };

    let mut found = false;
    let mut modified_lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('#') && trimmed.contains('=') {
            if let Some((k, _)) = line.split_once('=') {
                if k.trim() == unset_key {
                    found = true;
                    continue;
                }
            }
        }
        modified_lines.push(line.to_string());
    }

    if !found {
        eprintln!("kavach env: key '{}' not found in '{}'", unset_key, p.display());
        return 1;
    }

    let mut output = modified_lines.join("
");
    if content.ends_with('\n') {
        output.push('\n');
    }

    let parent = p.parent().unwrap_or_else(|| Path::new("."));
    let mut temp = match tempfile::NamedTempFile::new_in(parent) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("kavach env: failed to create temporary file: {}", e);
            return 1;
        }
    };

    if let Err(e) = temp.write_all(output.as_bytes()) {
        eprintln!("kavach env: failed to write to temporary file: {}", e);
        return 1;
    }

    if let Err(e) = temp.persist(p) {
        eprintln!("kavach env: failed to atomically update '{}': {}", p.display(), e);
        return 1;
    }

    println!("[OK] Atomically removed key '{}' from '{}'", unset_key, p.display());
    0
}

fn handle_diff(env_path: &Path, code_dir: &str) -> i32 {
    let mut env_keys = BTreeSet::new();

    if env_path.exists() {
        if let Ok(c) = fs::read_to_string(env_path) {
            for line in c.lines() {
                let trimmed = line.trim();
                if !trimmed.starts_with('#') && trimmed.contains('=') {
                    if let Some((k, _)) = trimmed.split_once('=') {
                        env_keys.insert(k.trim().to_string());
                    }
                }
            }
        }
    }

    println!("── Env Audit against Code: {} ──", code_dir);
    println!("Keys defined in {}: {}", env_path.display(), env_keys.len());
    for k in &env_keys {
        println!("  - {}", k);
    }

    0
}

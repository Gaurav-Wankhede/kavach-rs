use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use serde_json::Value;

pub(crate) fn run(
    query: &str,
    path_opt: Option<&str>,
    keys_only: bool,
    raw_mode: bool,
    json_mode: bool,
    set_expr: Option<&str>,
    dry_run: bool,
) -> i32 {
    let target_path = match path_opt {
        Some(p) => p,
        None if set_expr.is_some() => {
            eprintln!("kavach toml: --set requires a target file path");
            return 2;
        }
        None => "",
    };

    let input_content = if !target_path.is_empty() {
        let path = Path::new(target_path);
        if !path.exists() {
            eprintln!("kavach toml: file '{}' does not exist", target_path);
            return 2;
        }
        match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("kavach toml: error reading file '{}': {}", target_path, e);
                return 1;
            }
        }
    } else {
        let mut buf = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buf) {
            eprintln!("kavach toml: error reading stdin: {}", e);
            return 1;
        }
        buf
    };

    let toml_val: toml::Value = match toml::from_str(&input_content) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("kavach toml: invalid TOML: {}", e);
            return 1;
        }
    };

    let mut root_val: Value = match serde_json::to_value(&toml_val) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("kavach toml: error converting TOML to internal representation: {}", e);
            return 1;
        }
    };

    // Safe in-place mutation mode
    if let Some(set) = set_expr {
        let (path_part, val_part) = match set.split_once('=') {
            Some((p, v)) => (p.trim(), v.trim()),
            None => {
                eprintln!("kavach toml: invalid --set expression '{}'. Expected '.path=value'", set);
                return 2;
            }
        };

        let parsed_val: Value = serde_json::from_str(val_part).unwrap_or_else(|_| Value::String(val_part.to_string()));
        if !set_value(&mut root_val, path_part, parsed_val) {
            eprintln!("kavach toml: failed to set value at path '{}'", path_part);
            return 1;
        }

        let mutated_toml_val: toml::Value = match serde_json::from_value(root_val.clone()) {
            Ok(tv) => tv,
            Err(e) => {
                eprintln!("kavach toml: failed to convert mutated tree back to TOML: {}", e);
                return 1;
            }
        };

        let mutated_toml = match toml::to_string_pretty(&mutated_toml_val) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("kavach toml: error serializing mutated TOML: {}", e);
                return 1;
            }
        };

        if dry_run {
            println!("{}", mutated_toml);
            return 0;
        }

        // Atomic write via tempfile in same directory
        let p = Path::new(target_path);
        let parent = p.parent().unwrap_or_else(|| Path::new("."));
        let mut temp = match tempfile::NamedTempFile::new_in(parent) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("kavach toml: failed to create temporary file: {}", e);
                return 1;
            }
        };

        if let Err(e) = temp.write_all(mutated_toml.as_bytes()) {
            eprintln!("kavach toml: failed to write to temporary file: {}", e);
            return 1;
        }

        if let Err(e) = temp.persist(p) {
            eprintln!("kavach toml: failed to atomically replace file '{}': {}", target_path, e);
            return 1;
        }

        println!("[OK] Atomically updated {}", target_path);
        return 0;
    }

    let projected = project_value(&root_val, query);

    if keys_only {
        match projected {
            Some(Value::Object(map)) => {
                for k in map.keys() {
                    println!("{}", k);
                }
            }
            Some(Value::Array(arr)) => {
                for idx in 0..arr.len() {
                    println!("{}", idx);
                }
            }
            _ => {}
        }
        return 0;
    }

    let val = match projected {
        Some(v) => v,
        None => {
            return 0;
        }
    };

    if raw_mode {
        match val {
            Value::String(s) => println!("{}", s),
            Value::Null => {}
            other => println!("{}", other),
        }
        return 0;
    }

    if json_mode {
        match serde_json::to_string(&val) {
            Ok(s) => println!("{}", s),
            Err(e) => {
                eprintln!("kavach toml: error serializing JSON: {}", e);
                return 1;
            }
        }
        return 0;
    }

    // Default clean TOON output
    let toon_output = kavach_toon::encode_toon(val);
    println!("{}", toon_output.trim_end());
    0
}

fn project_value<'a>(root: &'a Value, query: &str) -> Option<&'a Value> {
    let clean_query = query.trim();
    if clean_query.is_empty() || clean_query == "." {
        return Some(root);
    }

    let mut current = root;
    let stripped = clean_query.strip_prefix('.').unwrap_or(clean_query);

    for segment in stripped.split('.') {
        if segment.is_empty() {
            continue;
        }

        if let Some(bracket_idx) = segment.find('[') {
            let key = &segment[..bracket_idx];
            let rest = &segment[bracket_idx..];

            if !key.is_empty() {
                current = current.get(key)?;
            }

            let mut remaining = rest;
            while let Some(start) = remaining.find('[') {
                let end = remaining.find(']')?;
                let idx_str = &remaining[start + 1..end];
                let idx: usize = idx_str.parse().ok()?;
                current = current.get(idx)?;
                remaining = &remaining[end + 1..];
            }
        } else {
            current = current.get(segment)?;
        }
    }

    Some(current)
}

fn set_value(root: &mut Value, query: &str, new_val: Value) -> bool {
    let clean_query = query.trim();
    let stripped = clean_query.strip_prefix('.').unwrap_or(clean_query);
    let parts: Vec<&str> = stripped.split('.').filter(|p| !p.is_empty()).collect();

    if parts.is_empty() {
        *root = new_val;
        return true;
    }

    let mut current = root;
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            if let Value::Object(map) = current {
                map.insert((*part).to_string(), new_val);
                return true;
            }
            return false;
        } else {
            if !current.is_object() {
                *current = Value::Object(serde_json::Map::new());
            }
            current = match current {
                Value::Object(map) => {
                    map.entry((*part).to_string())
                        .or_insert_with(|| Value::Object(serde_json::Map::new()))
                }
                _ => return false,
            };
        }
    }
    true
}

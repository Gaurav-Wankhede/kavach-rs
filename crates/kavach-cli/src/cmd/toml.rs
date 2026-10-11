use std::fs;
use std::io::{self, Read};
use std::path::Path;
use serde_json::Value;

pub(crate) fn run(
    query: &str,
    path_opt: Option<&str>,
    keys_only: bool,
    raw_mode: bool,
    json_mode: bool,
) -> i32 {
    let target_path = path_opt.unwrap_or("");

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

    let root_val: Value = match serde_json::to_value(&toml_val) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("kavach toml: error converting TOML to internal representation: {}", e);
            return 1;
        }
    };


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


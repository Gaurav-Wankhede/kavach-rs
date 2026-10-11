//! Lossless Tabular JSON Compactor (SmartCrusher principle).
use serde_json::Value;
use std::fmt::Write as _;

#[cfg(test)]
#[path = "tabular_test.rs"]
mod tests;

/// Errors that can occur when converting tabular text back to JSON.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TabularError {
    #[error("invalid header format")]
    InvalidHeader,
    #[error("mismatched column count at row {0}")]
    ColumnMismatch(usize),
}

/// Convert a JSON array of uniform objects into a dense, token-saving TSV table.
#[must_use]
/// Encode any serde_json::Value into official TOON (Token-Oriented Object Notation).
pub fn encode_toon(val: &Value) -> String {
    let mut out = String::new();
    encode_toon_value(val, 0, &mut out);
    out
}

fn encode_toon_value(val: &Value, indent: usize, out: &mut String) {
    let pad = " ".repeat(indent);
    match val {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(&b.to_string()),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => {
            if s.contains(',') || s.contains('\n') || s.contains(':') || s.contains('"') {
                out.push('"');
                out.push_str(&s.replace('\"', "\\\"").replace('"', "\""));
                out.push('"');
            } else {
                out.push_str(s);
            }
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                out.push_str("[]");
                return;
            }
            // Check if uniform array of objects for tabular form
            if let Some(first) = arr.first().and_then(Value::as_object) {
                let keys: Vec<&String> = first.keys().collect();
                let is_uniform = arr.iter().all(|item| {
                    if let Some(obj) = item.as_object() {
                        obj.len() == keys.len() && keys.iter().all(|k| obj.contains_key(*k))
                    } else {
                        false
                    }
                });
                if is_uniform && !keys.is_empty() {
                    let key_names: Vec<&str> = keys.iter().map(|k| k.as_str()).collect();
                    out.push_str(&format!("[{}]{{{{{}}}}}:\n", arr.len(), key_names.join(",")));
                    for item in arr {
                        let obj = item.as_object().unwrap();
                        out.push_str(&format!("{}  ", pad));
                        for (idx, k) in keys.iter().enumerate() {
                            if idx > 0 {
                                out.push(',');
                            }
                            let v = obj.get(*k).unwrap();
                            match v {
                                Value::String(s) => {
                                    if s.contains(',') || s.contains('\n') || s.contains('"') {
                                        out.push('"');
                                        out.push_str(&s.replace('"', "\""));
                                        out.push('"');
                                    } else {
                                        out.push_str(s);
                                    }
                                }
                                Value::Null => {}
                                _ => out.push_str(&v.to_string()),
                            }
                        }
                        out.push('\n');
                    }
                    return;
                }
            }

            // Primitive inline array or generic array
            let all_primitive = arr.iter().all(|v| !v.is_object() && !v.is_array());
            if all_primitive {
                out.push_str(&format!("[{}] ", arr.len()));
                for (idx, item) in arr.iter().enumerate() {
                    if idx > 0 {
                        out.push(',');
                    }
                    if let Value::String(s) = item {
                        if s.contains(',') || s.contains('"') {
                            out.push('"');
                            out.push_str(&s.replace('"', "\""));
                            out.push('"');
                        } else {
                            out.push_str(s);
                        }
                    } else {
                        out.push_str(&item.to_string());
                    }
                }
            } else {
                out.push_str(&format!("[{}]\n", arr.len()));
                for item in arr {
                    out.push_str(&format!("{}- ", pad));
                    encode_toon_value(item, indent + 2, out);
                    out.push('\n');
                }
            }
        }
        Value::Object(obj) => {
            if obj.is_empty() {
                out.push_str("{}");
                return;
            }
            let mut first = true;
            for (k, v) in obj {
                if !first {
                    out.push('\n');
                }
                first = false;
                out.push_str(&format!("{}{}: ", pad, k));
                if v.is_object() {
                    out.push('\n');
                    encode_toon_value(v, indent + 2, out);
                } else {
                    encode_toon_value(v, indent + 2, out);
                }
            }
        }
    }
}

pub fn json_to_table(items: &[Value]) -> Option<String> {
    if items.len() < 3 {
        return None;
    }

    let first = items.first()?.as_object()?;
    let keys: Vec<String> = first.keys().cloned().collect();
    if keys.is_empty() {
        return None;
    }

    // Verify all items are objects sharing identical keys.
    for item in items {
        let obj = item.as_object()?;
        if obj.len() != keys.len() || !keys.iter().all(|k| obj.contains_key(k)) {
            return None;
        }
    }

    let mut out = String::new();
    if writeln!(out, "[TABULAR_SCHEMA: {}]", keys.join("|")).is_err() {
        return None;
    }

    for item in items {
        let obj = item.as_object()?;
        let mut row_vals = Vec::with_capacity(keys.len());
        for k in &keys {
            let val = obj.get(k)?;
            let repr = match val {
                Value::Null => String::new(),
                Value::Bool(b) => b.to_string(),
                Value::Number(n) => n.to_string(),
                Value::String(s) => s.replace('\t', "\\t").replace('\n', "\\n"),
                _ => val.to_string(),
            };
            row_vals.push(repr);
        }
        out.push_str(&row_vals.join("\t"));
        out.push('\n');
    }

    Some(out)
}

/// Parse a dense tabular TSV string back into a JSON array of objects.
///
/// # Errors
/// Returns `TabularError` if the header is malformed or rows do not match schema.
pub fn table_to_json(table_str: &str) -> Result<Value, TabularError> {
    let mut lines = table_str.lines();
    let header = lines.next().ok_or(TabularError::InvalidHeader)?;
    if !header.starts_with("[TABULAR_SCHEMA: ") || !header.ends_with(']') {
        return Err(TabularError::InvalidHeader);
    }

    let cols_str = header
        .trim_start_matches("[TABULAR_SCHEMA: ")
        .trim_end_matches(']');
    let cols: Vec<&str> = cols_str.split('|').collect();
    if cols.is_empty() {
        return Err(TabularError::InvalidHeader);
    }

    let mut rows = Vec::new();
    for (row_idx, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() != cols.len() {
            return Err(TabularError::ColumnMismatch(row_idx.saturating_add(1)));
        }

        let mut obj = serde_json::Map::new();
        for (col, val_str) in cols.iter().zip(parts.iter()) {
            let parsed_val = if val_str.is_empty() {
                Value::Null
            } else if let Ok(b) = val_str.parse::<bool>() {
                Value::Bool(b)
            } else if let Ok(num) = val_str.parse::<i64>() {
                Value::Number(num.into())
            } else if let Ok(num) = val_str.parse::<f64>() {
                serde_json::Number::from_f64(num)
                    .map_or_else(|| Value::String((*val_str).to_owned()), Value::Number)
            } else {
                Value::String(val_str.replace("\\t", "\t").replace("\\n", "\n"))
            };
            obj.insert((*col).to_owned(), parsed_val);
        }
        rows.push(Value::Object(obj));
    }

    Ok(Value::Array(rows))
}

/// Compact a raw JSON array string if it meets tabular criteria and yields savings.
#[must_use]
pub fn compact_tabular(json_str: &str, min_savings_percent: usize) -> Option<String> {
    let val: Value = serde_json::from_str(json_str).ok()?;
    let arr = val.as_array()?;
    let table = json_to_table(arr)?;

    let orig_len = json_str.len();
    let new_len = table.len();
    if orig_len == 0 || new_len >= orig_len {
        return None;
    }

    let saved = orig_len.saturating_sub(new_len);
    let pct = saved.saturating_mul(100).checked_div(orig_len).unwrap_or(0);
    (pct >= min_savings_percent).then_some(table)
}

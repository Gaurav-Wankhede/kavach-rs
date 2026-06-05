//! Derive query keywords from a frontend file's path segments and content head.
use std::path::Path;

/// Extract query keywords from the file path segments and first 20 lines of content.
pub(super) fn extract_query_keywords(file_path: &str, content: &str) -> Vec<String> {
    let mut kw: Vec<String> = Vec::new();
    let Some(stem_str) = Path::new(file_path).file_stem().and_then(|s| s.to_str()) else {
        return kw;
    };
    for part in stem_str.split(|c: char| !c.is_alphanumeric()) {
        let p = part.trim().to_lowercase();
        if p.len() > 2 {
            kw.push(p);
        }
    }
    if let Some(parent) = Path::new(file_path).parent().and_then(|p| p.to_str()) {
        for seg in parent.split('/') {
            for part in seg.split('-') {
                let p = part.trim().to_lowercase();
                if p.len() > 2 {
                    kw.push(p);
                }
            }
        }
    }
    for line in content.lines().take(20) {
        for token in line.split(|c: char| !c.is_alphanumeric() && c != '-') {
            let t = token.trim().to_lowercase();
            if t.len() > 3 && !is_noise(t.as_str()) {
                kw.push(t);
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    kw.retain(|k| seen.insert(k.clone()));
    kw
}

fn is_noise(t: &str) -> bool {
    matches!(
        t,
        "import"
            | "from"
            | "const"
            | "function"
            | "return"
            | "export"
            | "default"
            | "react"
            | "props"
            | "type"
            | "interface"
            | "class"
            | "string"
            | "number"
            | "boolean"
            | "void"
            | "null"
            | "true"
            | "false"
    )
}

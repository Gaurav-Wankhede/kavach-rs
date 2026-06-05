//! Detects code patterns from file paths and content.

use crate::patterns::all_patterns;
use kavach_patterns::{is_dockerfile, is_frontend_file, is_go_file, is_python_file, is_rust_file};

#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "pattern matching exhaustive, wire new variants at caller sites"
)]
pub enum PatternType {
    Language,
    Framework,
    Tool,
}

#[derive(Debug, Clone)]
#[expect(
    clippy::exhaustive_structs,
    reason = "constructed at pattern detection boundary"
)]
pub struct DetectedPattern {
    pub pattern_type: PatternType,
    pub name: String,
    pub confidence: f64,
    pub evidence: Vec<String>,
}

#[must_use]
pub fn detect_patterns(files: &[&str], content: &str) -> Vec<DetectedPattern> {
    let mut results = Vec::new();
    detect_languages(files, &mut results);
    detect_frameworks(files, content, &mut results);
    results.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

fn push_lang(out: &mut Vec<DetectedPattern>, name: &str, count: u32, total: f64) {
    if count > 0 {
        out.push(DetectedPattern {
            pattern_type: PatternType::Language,
            name: name.into(),
            confidence: {
                #[expect(
                    clippy::float_arithmetic,
                    reason = "safe floating point division in confidence calculation"
                )]
                let div = f64::from(count) / total;
                div.min(1.0)
            },
            evidence: vec![format!("{count} {name} files found")],
        });
    }
}

fn detect_languages(files: &[&str], out: &mut Vec<DetectedPattern>) {
    let (mut rs, mut go, mut py, mut fe) = (0u32, 0u32, 0u32, 0u32);
    for f in files {
        if is_rust_file(f) {
            rs = rs.saturating_add(1);
        }
        if is_go_file(f) {
            go = go.saturating_add(1);
        }
        if is_python_file(f) {
            py = py.saturating_add(1);
        }
        if is_frontend_file(f) {
            fe = fe.saturating_add(1);
        }
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "file count cast to f64 for confidence ratios"
    )]
    let total = files.len().max(1) as f64;
    push_lang(out, "rust", rs, total);
    push_lang(out, "go", go, total);
    push_lang(out, "python", py, total);
    push_lang(out, "typescript", fe, total);
}

fn detect_frameworks(files: &[&str], content: &str, out: &mut Vec<DetectedPattern>) {
    for pat in all_patterns() {
        let file_hit = files.iter().any(|f| {
            let fl = f.to_lowercase();
            pat.file_indicators
                .iter()
                .any(|ind| fl.contains(&ind.to_lowercase()))
        });
        let content_hits: Vec<String> = pat
            .content_indicators
            .iter()
            .filter(|ind| content.contains(*ind))
            .map(|ind| format!("matched: {ind}"))
            .collect();
        if !file_hit && content_hits.is_empty() {
            continue;
        }
        let conf = match (file_hit, content_hits.len()) {
            (true, 0) => 0.50,
            (true, 1) => 0.80,
            (true, _) => 0.95,
            (false, 0 | 1) => 0.40,
            (false, _) => 0.75,
        };
        let mut evidence = content_hits;
        if file_hit {
            evidence.insert(0, "file indicator present".into());
        }
        if is_dockerfile(files.first().unwrap_or(&"")) && pat.name == "docker" {
            evidence.push("Dockerfile detected".into());
        }
        out.push(DetectedPattern {
            pattern_type: pat.pattern_type.clone(),
            name: pat.name.into(),
            confidence: conf,
            evidence,
        });
    }
}

#[cfg(test)]
#[path = "detector_tests.rs"]
mod tests;

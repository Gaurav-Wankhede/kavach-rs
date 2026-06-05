use crate::cli::SecurityAction;
use crate::cmd::io_safe::{into_exit_code, print_or_exit};
use kavach_patterns::security_scanner::{self, ScanResult};
use std::fs;
use std::path::Path;

const DEFAULT_OUTPUT: &str = ".kavach/security-scan.json";
const SKIP_DIRS: [&str; 3] = ["target", "node_modules", "dist"];

pub(super) fn run(action: SecurityAction) -> i32 {
    match action {
        SecurityAction::Init { path } => init(&path),
        SecurityAction::Scan { path, output } => scan(&path, output.as_deref()),
        SecurityAction::Process {
            input, batch_size, ..
        } => process(input.as_deref(), batch_size),
        SecurityAction::Report { format, output } => report_cmd(&format, output.as_deref()),
    }
}

fn init(path: &str) -> i32 {
    let kavach_dir = Path::new(path).join(".kavach");
    if let Err(e) = fs::create_dir_all(&kavach_dir) {
        eprintln!("kavach security init: failed to create .kavach dir: {e}");
        return 1;
    }

    let context_path = kavach_dir.join("security-context.md");
    let template = "# Security Context\n\n\
        ## Threat Model\n\
        - [ ] Authentication bypass\n\
        - [ ] Authorization escalation\n\
        - [ ] SQL injection\n\
        - [ ] XSS (stored/reflected)\n\
        - [ ] SSRF\n\
        - [ ] Command injection\n\n\
        ## Auth Flows\n\
        <!-- Document authentication flows here -->\n\n\
        ## Known False Positives\n\
        <!-- List patterns to ignore -->\n";

    if let Err(e) = fs::write(&context_path, template) {
        eprintln!("kavach security init: failed to write context: {e}");
        return 1;
    }

    let msg = format!("kavach security init: created {}", context_path.display());
    if let Err(io_err) = print_or_exit(&msg) {
        return into_exit_code(io_err);
    }
    0
}

// ALGO: recursive_walk with early-exit filter
// PROBLEM_CLASS: directory_traversal
// REJECTED: [{"name":"jwalk","reason":"parallel overhead for <10k files"},{"name":"ignore::WalkParallel","reason":"adds dependency, sequential sufficient"}]
// TIME: O(n) where n=files | SPACE: O(d) where d=max_depth
// YEAR: 2026 | SEARCHED: 2026-05
// TRADEOFF: sequential; parallel (jwalk) 4× faster for >100k files
// BENCHMARK: https://users.rust-lang.org/t/walkdir-performance-a-small-experiment/24112
fn scan(path: &str, output: Option<&str>) -> i32 {
    let output_path = output.map_or(DEFAULT_OUTPUT, |s| s);
    let root = Path::new(path);

    let kavach_dir = root.join(".kavach");
    if let Err(e) = fs::create_dir_all(&kavach_dir) {
        eprintln!("kavach security scan: failed to create .kavach dir: {e}");
        return 1;
    }

    let mut results: Vec<ScanResult> = Vec::new();
    let mut scanned = 0usize;
    let mut sensitive = 0usize;

    walk_dir(root, &mut results, &mut scanned, &mut sensitive);

    let total_findings: usize = results.iter().map(|r| r.findings.len()).sum();

    let json = match serde_json::to_string_pretty(&results) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("kavach security scan: JSON serialize failed: {e}");
            return 1;
        }
    };

    let out_path = if output_path.starts_with('/') {
        Path::new(output_path).to_path_buf()
    } else {
        root.join(output_path)
    };

    if let Some(parent) = out_path.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        eprintln!("kavach security scan: failed to create output dir: {e}");
        return 1;
    }

    if let Err(e) = fs::write(&out_path, &json) {
        eprintln!("kavach security scan: failed to write output: {e}");
        return 1;
    }

    let msg = format!(
        "kavach security scan: scanned {scanned} files, {sensitive} security-sensitive, {total_findings} findings → {}",
        out_path.display()
    );
    if let Err(io_err) = print_or_exit(&msg) {
        return into_exit_code(io_err);
    }
    0
}

fn walk_dir(dir: &Path, results: &mut Vec<ScanResult>, scanned: &mut usize, sensitive: &mut usize) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };

        if name.starts_with('.') || SKIP_DIRS.contains(&name) {
            continue;
        }

        if ft.is_dir() {
            walk_dir(&path, results, scanned, sensitive);
        } else if !ft.is_dir() && security_scanner::is_security_sensitive(&path) {
            *sensitive = sensitive.saturating_add(1);
            *scanned = scanned.saturating_add(1);
            if let Ok(content) = fs::read_to_string(&path) {
                let result = security_scanner::scan_file(&path.to_string_lossy(), &content);
                if !result.findings.is_empty() {
                    results.push(result);
                }
            }
        }
    }
}

#[expect(
    clippy::arithmetic_side_effects,
    clippy::integer_division,
    reason = "bounded calculation: (total + batch_size-1) / batch_size is ceiling division with safe bounds"
)]
fn process(input: Option<&str>, batch_size: usize) -> i32 {
    if batch_size == 0 {
        eprintln!("kavach security process: --batch-size must be >= 1");
        return 1;
    }
    let input_path = input.map_or(DEFAULT_OUTPUT, |s| s);

    let content = match fs::read_to_string(input_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("kavach security process: failed to read {input_path}: {e}");
            return 1;
        }
    };

    let results: Vec<ScanResult> = match serde_json::from_str(&content) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("kavach security process: JSON parse failed: {e}");
            return 1;
        }
    };

    let total: usize = results.iter().map(|r| r.findings.len()).sum();
    let batches = total.saturating_add(batch_size.saturating_sub(1)) / batch_size.max(1);

    let summary = format!(
        "kavach security process: {total} findings in {} files, {batches} batches of {batch_size}",
        results.len()
    );
    if let Err(io_err) = print_or_exit(&summary) {
        return into_exit_code(io_err);
    }
    if let Err(io_err) =
        print_or_exit("  LLM deep analysis not yet implemented — findings available for report")
    {
        return into_exit_code(io_err);
    }
    0
}

fn report_cmd(format: &str, output: Option<&str>) -> i32 {
    let content = match fs::read_to_string(DEFAULT_OUTPUT) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("kavach security report: failed to read {DEFAULT_OUTPUT}: {e}");
            eprintln!("  run `kavach security scan` first");
            return 1;
        }
    };

    let results: Vec<ScanResult> = match serde_json::from_str(&content) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("kavach security report: JSON parse failed: {e}");
            return 1;
        }
    };

    let report_content = match format {
        "json" => match serde_json::to_string_pretty(&results) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("kavach security report: JSON serialize failed: {e}");
                return 1;
            }
        },
        "markdown" => generate_markdown_report(&results),
        other => {
            eprintln!("kavach security report: unknown format '{other}', use 'markdown' or 'json'");
            return 1;
        }
    };

    match output {
        Some(path) => {
            if let Err(e) = fs::write(path, &report_content) {
                eprintln!("kavach security report: failed to write {path}: {e}");
                return 1;
            }
            let msg = format!("kavach security report: written to {path}");
            if let Err(io_err) = print_or_exit(&msg) {
                return into_exit_code(io_err);
            }
        }
        None => {
            if let Err(io_err) = print_or_exit(&report_content) {
                return into_exit_code(io_err);
            }
        }
    }
    0
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "bounded calculation: 256 + (results.len() * 128) capacity estimation for String"
)]
fn generate_markdown_report(results: &[ScanResult]) -> String {
    use std::fmt::Write;
    let total: usize = results.iter().map(|r| r.findings.len()).sum();
    let cap = 256 + results.len() * 128;
    let mut md = String::with_capacity(cap);

    writeln!(md, "# Security Scan Report\n").ok();
    writeln!(md, "**Total findings:** {total}\n").ok();

    for result in results {
        if result.findings.is_empty() {
            continue;
        }
        writeln!(md, "## {}\n", result.file).ok();

        for f in &result.findings {
            writeln!(
                md,
                "- **L{}** [{}/{}]: {}\n  - Fix: {}\n",
                f.line, f.severity, f.category, f.pattern, f.fix
            )
            .ok();
        }
    }

    md
}

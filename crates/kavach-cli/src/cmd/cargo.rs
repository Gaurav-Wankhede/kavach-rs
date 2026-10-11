use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};

pub(crate) fn run(subcommand: &str, args: &[String]) -> i32 {
    let mut actual_subcommand = subcommand.to_string();
    let mut actual_args: Vec<String> = args.to_vec();

    // Map "test" or "nextest" directly to cargo nextest run
    let is_test = if subcommand == "test" {
        actual_subcommand = "nextest".to_string();
        if actual_args.is_empty() || actual_args[0] != "run" {
            actual_args.insert(0, "run".to_string());
        }
        true
    } else if subcommand == "nextest" {
        true
    } else {
        false
    };

    let mut cmd = Command::new("cargo");
    cmd.arg(&actual_subcommand);
    cmd.args(&actual_args);

    if is_test {
        cmd.env("CI", "1");
        cmd.env("NEXTEST_COLOR", "never");
    }

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("kavach cargo: failed to spawn cargo {}: {}", subcommand, e);
            return 1;
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let out_thread = std::thread::spawn(move || {
        let mut lines = Vec::new();
        if let Some(out) = stdout {
            let reader = BufReader::new(out);
            for line_res in reader.lines() {
                if let Ok(line) = line_res {
                    lines.push(line);
                }
            }
        }
        lines
    });

    let err_thread = std::thread::spawn(move || {
        let mut lines = Vec::new();
        if let Some(err) = stderr {
            let reader = BufReader::new(err);
            for line_res in reader.lines() {
                if let Ok(line) = line_res {
                    lines.push(line);
                }
            }
        }
        lines
    });

    let status = match child.wait() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("kavach cargo: error waiting on child process: {}", e);
            return 1;
        }
    };

    let stdout_lines = out_thread.join().unwrap_or_default();
    let stderr_lines = err_thread.join().unwrap_or_default();

    if is_test {
        render_test_output(status.success(), &stdout_lines, &stderr_lines);
    } else {
        render_compiler_output(subcommand, status.success(), &stdout_lines, &stderr_lines);
    }

    status.code().unwrap_or(if status.success() { 0 } else { 1 })
}

fn render_test_output(success: bool, stdout_lines: &[String], stderr_lines: &[String]) {
    let mut passed_count = 0usize;
    let mut failed_count = 0usize;
    let mut summary_line = None;
    let mut failures = Vec::new();

    let all_lines = stderr_lines.iter().chain(stdout_lines.iter());
    for line in all_lines {
        let trimmed = line.trim();
        if trimmed.starts_with("PASS") {
            passed_count += 1;
        } else if trimmed.starts_with("FAIL") {
            failed_count += 1;
            failures.push(trimmed.to_string());
        } else if trimmed.contains("Summary [") {
            summary_line = Some(trimmed.to_string());
        } else if trimmed.starts_with("error:") || trimmed.starts_with("failures:") {
            failures.push(trimmed.to_string());
        }
    }
    let _ = failed_count;

    if success {
        if let Some(s) = summary_line {
            println!("[CARGO:NEXTEST] ok: {}", s);
        } else {
            println!("[CARGO:NEXTEST] ok: {} passed", passed_count);
        }
    } else {
        println!("[CARGO:NEXTEST] FAILED:");
        for f in failures.iter().take(20) {
            println!("    {}", f);
        }
        if let Some(s) = summary_line {
            println!("  {}", s);
        }
    }
}

fn render_compiler_output(subcommand: &str, success: bool, stdout_lines: &[String], stderr_lines: &[String]) {
    let mut compiled_count = 0usize;
    let mut finished_line = None;
    let mut warnings_and_errors = Vec::new();

    let all_lines = stderr_lines.iter().chain(stdout_lines.iter());
    let mut capture_details = false;

    for line in all_lines {
        let trimmed = line.trim();
        if trimmed.starts_with("Compiling ") || trimmed.starts_with("Checking ") {
            compiled_count += 1;
            capture_details = false;
        } else if trimmed.starts_with("Finished ") {
            finished_line = Some(trimmed.to_string());
            capture_details = false;
        } else if trimmed.starts_with("warning:") || trimmed.starts_with("error") {
            capture_details = true;
            warnings_and_errors.push(line.clone());
        } else if capture_details {
            warnings_and_errors.push(line.clone());
        }
    }

    if success {
        let target_summary = finished_line.unwrap_or_else(|| "Done".to_string());
        if warnings_and_errors.is_empty() {
            println!("[CARGO:{}] ok ({} crates processed) - {}", subcommand.to_uppercase(), compiled_count, target_summary);
        } else {
            println!("[CARGO:{}] ok with warnings ({} crates processed) - {}", subcommand.to_uppercase(), compiled_count, target_summary);
            for w in warnings_and_errors.iter().take(30) {
                println!("{}", w);
            }
        }
    } else {
        println!("[CARGO:{}] FAILED ({} crates processed):", subcommand.to_uppercase(), compiled_count);
        for e in warnings_and_errors.iter().take(40) {
            println!("{}", e);
        }
    }
}

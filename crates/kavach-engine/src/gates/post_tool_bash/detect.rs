//! Pure output/command classifiers: test command, empty suite, package install,
//! package-not-found, port conflict.

/// Detect if a bash command is a test execution.
pub(super) fn is_test_command(cmd: &str) -> bool {
    let test_patterns = [
        "cargo test",
        "cargo nextest",
        "bun test",
        "bun run test",
        "bunx playwright",
        "bunx --bun playwright",
        "npx playwright",
        "npx jest",
        "pytest",
        "go test",
        "mix test",
    ];
    test_patterns.iter().any(|p| cmd.contains(p))
}

/// Detect empty test suites: "0 passed; 0 failed" means no tests ran.
pub(super) fn is_empty_test_suite(output: Option<&str>) -> bool {
    let text = match output {
        Some(t) if !t.is_empty() => t,
        _ => return false,
    };
    text.contains("0 passed; 0 failed") && !text.contains("1 passed") && !text.contains("2 passed")
}

/// Detect if a bash command is a package install.
pub(super) fn is_package_install(cmd: &str) -> bool {
    let patterns = [
        "bun add",
        "bun install",
        "npm install",
        "npm i ",
        "yarn add",
        "pnpm add",
        "cargo add",
    ];
    patterns.iter().any(|p| cmd.contains(p))
}

/// Detect if package install output indicates not found (404).
pub(super) fn is_package_not_found(output: Option<&str>) -> bool {
    let text = match output {
        Some(t) if !t.is_empty() => t,
        _ => return false,
    };
    text.contains("404")
        || text.contains("not found")
        || text.contains("No matching version")
        || text.contains("no such package")
}

/// Detect port conflict errors and extract the port number.
/// Matches: EADDRINUSE, "address already in use", "port X in use".
pub(super) fn detect_port_conflict(output: Option<&str>) -> Option<u16> {
    let text = match output {
        Some(t) if !t.is_empty() => t,
        _ => return None,
    };
    if !text.contains("EADDRINUSE")
        && !text.contains("address already in use")
        && !text.contains("port")
        && !text.contains("in use")
    {
        return None;
    }
    for line in text.lines() {
        if line.contains("EADDRINUSE") || line.contains("address already in use") {
            // Extract port from end of line (after last colon).
            if let Some(pos) = line.rfind(':') {
                let port_str = line.get(pos.saturating_add(1)..).unwrap_or("").trim();
                if let Ok(port) = port_str.parse::<u16>() {
                    return Some(port);
                }
            }
        }
        // Pattern: "port XXXX is already in use".
        if line.contains("port") && line.contains("in use") {
            for word in line.split_whitespace() {
                if let Ok(port) = word.parse::<u16>() {
                    return Some(port);
                }
            }
        }
    }
    None
}

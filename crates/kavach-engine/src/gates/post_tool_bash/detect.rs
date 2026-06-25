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

/// A failed `kavach` invocation classified from clap's stderr (decision.kavach-misuse-recovery).
#[derive(Debug, PartialEq, Eq)]
pub(super) enum KavachMisuse {
    StaleBinary,
    UnknownVerb,
    UnknownFlag,
}

/// Shipped top-level verbs (source of truth: `kavach-cli/src/cli.rs::Commands`); a drift test guards this.
const KAVACH_VERBS: &[&str] = &[
    "status", "web", "servers", "gates", "session", "rules", "db", "install", "heal",
    "loophole", "schema", "ask", "oversized", "tailwind-plus", "doctor", "phase", "spec",
    "loop", "verify", "deploy", "verify-frontend", "pipeline", "security", "todos", "tasks",
    "context", "mistake", "bulk", "goal", "bg", "team", "think", "toolbelt", "lint", "commands",
];

/// Classify a failed `kavach` command from its clap-error output. Returns `None`
/// when the command is not a `kavach` call, did not fail with a clap usage error,
/// or its error is a runtime/DB error (not the agent's fault to fix by guessing).
pub(super) fn classify_kavach_misuse(cmd: &str, output: Option<&str>) -> Option<KavachMisuse> {
    let trimmed = cmd.trim_start();
    if !(trimmed == "kavach" || trimmed.starts_with("kavach ")) {
        return None;
    }
    let text = match output {
        Some(t) if !t.is_empty() => t,
        _ => return None,
    };
    let unknown_verb = text.contains("unrecognized subcommand")
        || text.contains("invalid subcommand");
    let unknown_flag = text.contains("unexpected argument")
        || text.contains("unexpected value")
        || text.contains("argument was not expected");
    if !unknown_verb && !unknown_flag {
        return None;
    }
    if rejected_token_is_known_verb(text) {
        return Some(KavachMisuse::StaleBinary);
    }
    if unknown_verb {
        return Some(KavachMisuse::UnknownVerb);
    }
    Some(KavachMisuse::UnknownFlag)
}

/// True if a token clap quoted as rejected is a real shipped verb — the
/// stale-binary fingerprint (source has it, the running binary does not).
fn rejected_token_is_known_verb(text: &str) -> bool {
    text.split('\'')
        .filter(|tok| !tok.contains(char::is_whitespace) && !tok.is_empty())
        .map(|tok| tok.trim_start_matches("--"))
        .any(|tok| KAVACH_VERBS.contains(&tok))
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

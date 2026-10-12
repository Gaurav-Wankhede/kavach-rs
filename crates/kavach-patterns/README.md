# `kavach-patterns`

> **Quote-Aware POSIX Parsing & Multi-Pattern Anti-Pattern Detection for Autonomous AI Coding Agent Sandboxes.**

[![Crates.io](https://img.shields.io/crates/v/kavach-patterns.svg)](https://crates.io/crates/kavach-patterns)
[![Documentation](https://docs.rs/kavach-patterns/badge.svg)](https://docs.rs/kavach-patterns)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

---

## What is `kavach-patterns`?

`kavach-patterns` is a high-performance pattern matching and command inspection crate designed specifically for AI agent runtimes. It validates shell command lines, detects banned legacy tools (`cat`, `sed`, `grep`, `find`), detects prompt-injection attacks, and flags unverified assumptions before any command can be executed against the host operating system.

---

## Why Does It Exist? (The Problem It Solves)

Simple regex checks (`command.contains("cat")`) **always fail** in real-world AI coding agents:
1. **Shell Quoting Evasion:** An agent executing `echo "concatenate this"` or `git commit -m "cat"` will falsely trigger a naive string search.
2. **Heredoc & Redirection Bypasses:** An agent executing `cat << EOF > file.txt` bypasses `cat <file>` filters unless the command is parsed into real POSIX tokens.
3. **Fuzzy Typos & Assumptions:** Agents frequently guess CLI flags that do not exist, triggering endless error retry loops.

`kavach-patterns` solves this by combining exact POSIX shell tokenization, $O(n)$ multi-pattern automata, and string-distance metrics.

---

## Behind the Curtain: Honest Architectural Disclosures

We believe in complete transparency about what powers this crate under the hood:

- **Underlying Engines & Open Source Libraries:**
  - **[`shell-words`](https://crates.io/crates/shell-words):** Implements exact POSIX.1-2008 shell tokenization. Handles single quotes, double quotes, escape backslashes, and subshell boundaries correctly.
  - **[`aho-corasick`](https://crates.io/crates/aho-corasick):** Implements the Aho-Corasick string-searching automaton. Allows searching for hundreds of banned patterns in a single linear $O(n)$ pass through the command stream.
  - **[`strsim`](https://crates.io/crates/strsim):** Computes Levenshtein and Jaro-Winkler string similarities to detect near-miss duplicate task titles and anti-pattern typos.
- **What Kavach Builds on Top:**
  - The **Gate Verification Architecture:** Wraps raw tokens into semantic stop-gates (`banned_tool`, `prompt_injection`, `path_traversal`, `unverified_assumption`).
  - Strict allowlist/denylist policy mapping tailored to autonomous LLM coding harnesses (Claude Code, Gemini CLI, Cursor, Devin).
- **Tradeoffs & Non-Goals:**
  - This is an **in-process linguistic firewall**, NOT a Linux kernel seccomp or eBPF sandbox. It intercepts commands at the agent execution layer before syscall dispatch.

---

## Key Capabilities

- **POSIX-Compliant Tokenization:** Accurately distinguishes between executable binary names and quoted string arguments.
- **Single-Pass Multi-Pattern Matching:** Searches against 50+ banned legacy tools simultaneously with microsecond latency.
- **Semantic Anti-Pattern Detection:** Identifies destructive agent behaviors such as blind directory guessing and whole-file dumping.

---

## Quickstart

Add to your `Cargo.toml`:
```toml
[dependencies]
kavach-patterns = "1.3"
```

Inspect command lines safely:
```rust
use kavach_patterns::posix::tokenize_command;

fn main() {
    let cmd = "kavach grep \"fn main\" src/ -C 2";
    let tokens = tokenize_command(cmd).expect("valid posix command");
    assert_eq!(tokens[0], "kavach");
    assert_eq!(tokens[1], "grep");
}
```

---

## Part of the Kavach Sovereign Engine

This crate is a foundational security component of **[`kavach-rs`](https://github.com/Gaurav-Wankhede/kavach-rs)**, the sovereign context firewall and cognitive memory substrate for AI coding agents.

- Companion Web Search: **[`webfind`](https://github.com/Gaurav-Wankhede/webfind)**
- Canonical Repository: [`https://github.com/Gaurav-Wankhede/kavach-rs`](https://github.com/Gaurav-Wankhede/kavach-rs)

---

## Author & License

- **Author & Architect:** [Gaurav Wankhede](https://github.com/Gaurav-Wankhede) (`gauravanilwankhede2002@gmail.com`)
- **License:** MIT License

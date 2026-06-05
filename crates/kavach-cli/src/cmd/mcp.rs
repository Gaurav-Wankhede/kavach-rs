// kavach mcp — Model Context Protocol stdio server bridging Claude Code
// to the kavach-rpc backend.
//
// ARCH: McpStdioBridge
// PROBLEM_CLASS: cli-protocol-bridge
// SCOPE: process | CAP: AP | SEARCHED: 2026-05
// REJECTED: [{"name":"direct kavach-rpc registration","reason":"kavach rpc speaks JSON-RPC 2.0 plain, not MCP lifecycle (initialize/tools/list/tools/call)"}]
// TIME: O(1) per request; stream-based
// SPACE: O(1) — no message buffering
// YEAR: 2026
// TRADEOFF: thin wrapper; doesn't gain free auth/streaming from a full SDK,
//   but adds zero new deps and ships in 200 LOC vs ~600 with rmcp.
//
// SOURCE: github.com/modelcontextprotocol/modelcontextprotocol — protocol spec.
// SOURCE: modelcontextprotocol.io/docs/learn/architecture — lifecycle:
//   initialize → notifications/initialized → tools/list → tools/call.
// SOURCE: code.claude.com/docs/en/mcp — Claude Code MCP transport contract.
// SOURCE: ~/.claude/CLAUDE.md §LSP-FIRST + Anthropic blog "How Claude Code
//   works in large codebases" — MCP is the canonical external-system layer;
//   kavach-rs previously shelled out to `kavach db` everywhere, an audited
//   misalignment with the layered-harness best practice.
//
// Wire to Claude Code:
//   claude mcp add kavach -- kavach mcp
// Then in any session: tools/list shows kavach.* tools backed by kavach-db.

use std::io::{BufRead, Write};

use serde_json::{Value, json};

const MCP_PROTOCOL_VERSION: &str = "2025-11-25";
const SERVER_NAME: &str = "kavach";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) fn run() -> i32 {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    // FIX-H (cold reviewer): MCP lifecycle state machine. The spec requires
    // `initialize` to be the FIRST request; `tools/list` / `tools/call` are
    // only valid AFTER `notifications/initialized` (which the client sends
    // post-handshake). Without this guard, a misbehaving client could call
    // tools/call before initialize and get a tool execution.
    // SOURCE: modelcontextprotocol.io/docs/learn/architecture lifecycle:
    //   initialize → response → notifications/initialized → tools/* allowed.
    let mut initialized = false;

    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        // FIX [I2 reviewer cold-cluster] malformed JSON used to silently
        // `continue`, leaving the client hanging waiting for a response that
        // would never come. Per JSON-RPC 2.0 + MCP spec, parse errors return
        // code -32700 with id=null.
        // SOURCE: jsonrpc.org/specification §5.1 — Parse error = -32700.
        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": {
                        "code": -32700,
                        "message": format!("Parse error: {e}")
                    }
                });
                if writeln!(out, "{err_resp}").is_err() {
                    return 1;
                }
                // MCP transport requires the framed response be visible to the
                // client immediately; flush failure means the client never sees
                // the parse error — exit instead of silently continuing.
                if out.flush().is_err() {
                    return 1;
                }
                continue;
            }
        };
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id = req.get("id").cloned();
        let response = match method {
            "initialize" => handle_initialize(id),
            "notifications/initialized" => {
                initialized = true;
                continue; // notification: no response
            }
            "tools/list" if initialized => handle_tools_list(id),
            "tools/call" if initialized => handle_tools_call(id, req.get("params").cloned()),
            "ping" => json!({"jsonrpc": "2.0", "id": id, "result": {}}),
            "tools/list" | "tools/call" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32002,
                    "message": "MCP lifecycle violation: tools/* called before notifications/initialized"
                }
            }),
            _ => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": format!("method not found: {method}")}
            }),
        };
        if writeln!(out, "{response}").is_err() {
            return 1;
        }
        if out.flush().is_err() {
            return 1;
        }
    }
    0
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "JSON-RPC handler: id is owned and serialized into the json! response; by-value matches the dispatch match that hands off req.get(\"id\").cloned()"
)]
fn handle_initialize(id: Option<Value>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": {"tools": {}},
            "serverInfo": {"name": SERVER_NAME, "version": SERVER_VERSION}
        }
    })
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "JSON-RPC handler: id is owned and serialized into the json! response; by-value matches the dispatch match"
)]
fn handle_tools_list(id: Option<Value>) -> Value {
    // Six core kavach tools surface as MCP. Each is a thin wrapper around
    // the corresponding `kavach db ...` subcommand. Adding more later is
    // additive — the dispatcher in handle_tools_call is the only edit point.
    let tools = json!([
        {
            "name": "kavach_db_query",
            "description": "Query kavach memory entries for a project + category.",
            "inputSchema": {
                "type": "object",
                "required": ["project", "category"],
                "properties": {
                    "project": {"type": "string"},
                    "category": {"type": "string"}
                }
            }
        },
        {
            "name": "kavach_db_get",
            "description": "Fetch a single kavach memory entry by key.",
            "inputSchema": {
                "type": "object",
                "required": ["project", "category", "key"],
                "properties": {
                    "project": {"type": "string"},
                    "category": {"type": "string"},
                    "key": {"type": "string"},
                    "full": {"type": "boolean"}
                }
            }
        },
        {
            "name": "kavach_db_kanban",
            "description": "Show kanban board for a project.",
            "inputSchema": {
                "type": "object",
                "required": ["project"],
                "properties": {"project": {"type": "string"}}
            }
        },
        {
            "name": "kavach_mistake_list",
            "description": "List top-N K-PRI mistake-ledger rows for a project.",
            "inputSchema": {
                "type": "object",
                "required": ["project"],
                "properties": {
                    "project": {"type": "string"},
                    "limit": {"type": "integer"},
                    "gate": {"type": "string"}
                }
            }
        },
        {
            "name": "kavach_mistake_stats",
            "description": "Hit-count distribution + per-gate breakdown.",
            "inputSchema": {
                "type": "object",
                "required": ["project"],
                "properties": {"project": {"type": "string"}}
            }
        },
        {
            "name": "kavach_pipeline_status",
            "description": "Show multi-stage pipeline status for a project.",
            "inputSchema": {
                "type": "object",
                "required": ["project"],
                "properties": {"project": {"type": "string"}}
            }
        }
    ]);
    json!({"jsonrpc": "2.0", "id": id, "result": {"tools": tools}})
}

fn handle_tools_call(id: Option<Value>, params: Option<Value>) -> Value {
    let Some(p) = params else {
        return tool_error(id, "missing params");
    };
    let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let args = p.get("arguments").cloned().unwrap_or_else(|| json!({}));
    let cmd_result = match name {
        "kavach_db_query" => vec_args(&["db", "query"], &args, &["project", "category"], &[]),
        "kavach_db_get" => vec_args(
            &["db", "get"],
            &args,
            &["project", "category", "key"],
            &["full"],
        ),
        "kavach_db_kanban" => vec_args(&["db", "kanban"], &args, &["project"], &[]),
        "kavach_mistake_list" => vec_args(
            &["mistake", "list"],
            &args,
            &["project"],
            &["limit", "gate"],
        ),
        "kavach_mistake_stats" => vec_args(&["mistake", "stats"], &args, &["project"], &[]),
        "kavach_pipeline_status" => vec_args(&["pipeline", "status"], &args, &["project"], &[]),
        _ => return tool_error(id, &format!("unknown tool: {name}")),
    };
    let cmd_argv = match cmd_result {
        Ok(v) => v,
        Err(e) => return tool_error(id, &e),
    };
    let out = std::process::Command::new("kavach")
        .args(&cmd_argv)
        .output();
    let Ok(o) = out else {
        return tool_error(id, "kavach binary not found in PATH");
    };
    // FIX [I1 reviewer cold-cluster] non-zero exit could merge stderr into
    // the result silently when stdout was empty, but stderr disappeared when
    // stdout had content. On failure ALWAYS surface stderr so the MCP client
    // sees the underlying kavach error, not a misleading partial stdout.
    let stdout = String::from_utf8_lossy(&o.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&o.stderr).into_owned();
    let success = o.status.success();
    let text = if success {
        if stdout.is_empty() { stderr } else { stdout }
    } else {
        // Failure path: include both streams so the client gets full context.
        match (stdout.is_empty(), stderr.is_empty()) {
            (true, true) => format!("kavach exited with {}", o.status),
            (true, false) => stderr,
            (false, true) => stdout,
            (false, false) => format!("stdout:\n{stdout}\n\nstderr:\n{stderr}"),
        }
    };
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [{"type": "text", "text": text}],
            "isError": !success
        }
    })
}

fn vec_args(
    head: &[&str],
    args: &Value,
    required: &[&str],
    optional: &[&str],
) -> Result<Vec<String>, String> {
    let mut cmd_argv: Vec<String> = head.iter().map(ToString::to_string).collect();
    for k in required {
        let v = args
            .get(k)
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("missing required arg: {k}"))?;
        cmd_argv.push(format!("--{k}"));
        cmd_argv.push(v.to_owned());
    }
    for k in optional {
        if let Some(v) = args.get(k) {
            if let Some(s) = v.as_str() {
                cmd_argv.push(format!("--{k}"));
                cmd_argv.push(s.to_owned());
            } else if v.as_bool() == Some(true) {
                cmd_argv.push(format!("--{k}"));
            } else if let Some(n) = v.as_i64() {
                cmd_argv.push(format!("--{k}"));
                cmd_argv.push(n.to_string());
            }
        }
    }
    Ok(cmd_argv)
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "JSON-RPC handler: id is owned and serialized into the json! response; by-value matches the dispatch match"
)]
fn tool_error(id: Option<Value>, msg: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": -32602, "message": msg}
    })
}

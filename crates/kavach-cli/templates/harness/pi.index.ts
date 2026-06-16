// Kavach native extension for the Pi coding agent (earendil-works/pi).
// Install to ~/.pi/agent/extensions/kavach/index.ts (global, auto-discovered) or
// .pi/extensions/kavach/index.ts (project). Pi loads this via pi.on(event, cb).
//
// Every gate shells out to the SAME kavach binary with --vendor pi: kavach lowers
// Pi's payload to its canonical pivot, runs the gate, and renders Pi's native
// return contract { block: true, reason? } (deny) or {} (allow). Pi blocks via the
// returned object, NOT an exit code. agent_end is Pi's Stop-equivalent, so the
// autonomous-loop AUTO_CONTINUE reaches Pi too.
// SOURCE: github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/extensions.md

import { spawnSync } from "node:child_process";

// The `kavach gates ` prefix below is pinned to the absolute binary path by
// `kavach install` at write time (it rewrites every "kavach gates " occurrence),
// so the installed extension resolves regardless of $PATH. The literal here is the
// dev/PATH fallback before pinning.
const KAVACH_GATES = "kavach gates ".trim();

/** Run one kavach gate with the event JSON on stdin; return parsed { block, reason }. */
function runGate(gate: string, payload: unknown): { block?: boolean; reason?: string } {
  // KAVACH_GATES is "<bin> gates" (pinned at install); split off the binary path.
  const parts = KAVACH_GATES.split(" ");
  const bin = parts[0] ?? "kavach";
  const res = spawnSync(bin, [...parts.slice(1), gate, "--hook", "--vendor", "pi"], {
    input: JSON.stringify(payload),
    encoding: "utf8",
  });
  // Fail CLOSED: if the binary is missing or errors, deny rather than silently allow.
  if (res.error || res.status === null) {
    return { block: true, reason: `kavach: gate ${gate} unavailable (${res.error?.message ?? "spawn failed"})` };
  }
  const out = (res.stdout ?? "").trim();
  if (!out) return {};
  try {
    return JSON.parse(out) as { block?: boolean; reason?: string };
  } catch {
    return { block: true, reason: "kavach: unparseable gate response (fail-closed)" };
  }
}

/** Map a Pi event payload onto the canonical CC-shaped hook input kavach expects. */
function toHookInput(event: string, ev: Record<string, unknown>): Record<string, unknown> {
  return {
    hook_event_name: event,
    tool_name: ev.tool_name ?? ev.tool ?? "",
    tool_input: ev.args ?? ev.tool_input ?? {},
    cwd: ev.cwd ?? process.cwd(),
    session_id: ev.session_id ?? ev.sessionId ?? "",
  };
}

// Pi extension entrypoint: receives the extension API and registers lifecycle hooks.
export default function activate(pi: {
  on: (event: string, cb: (ev: Record<string, unknown>, ctx?: unknown) => unknown) => void;
}): void {
  // tool_call -> PreToolUse: handler is awaited, so returning { block: true } stops
  // the tool before it runs (no fail-open race). Returning {} / nothing allows.
  pi.on("tool_call", async (ev) => {
    const r = runGate("pre-tool", toHookInput("PreToolUse", ev));
    return r.block ? { block: true, reason: r.reason } : undefined;
  });

  // tool_result -> PostToolUse: observation only (research capture); never blocks.
  pi.on("tool_result", async (ev) => {
    runGate("post-tool", toHookInput("PostToolUse", ev));
  });

  // session_start -> SessionStart: restore state + inject context (incl. temporal awareness).
  pi.on("session_start", async (ev) => {
    runGate("session-start", toHookInput("SessionStart", ev));
  });

  // session_before_compact -> PreCompact: relay custom instructions before compaction.
  pi.on("session_before_compact", async (ev) => {
    runGate("pre-compact", toHookInput("PreCompact", ev));
  });

  // agent_end -> Stop: Pi's turn-end. Drives the autonomous loop — a block return
  // carries [AUTO_CONTINUE] so Pi keeps working on the next kanban card.
  pi.on("agent_end", async (ev) => {
    const r = runGate("stop", toHookInput("Stop", ev));
    return r.block ? { block: true, reason: r.reason } : undefined;
  });
}

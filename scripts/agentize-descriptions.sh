#!/usr/bin/env bash
# Convert the 6 passive "Level N - role" global-agent descriptions to action
# imperatives so the kavach ranker can dispatch each as [INVOKE_AGENT]. Idempotent.
# SOURCE: decision.harness.agents-as-action-imperatives.
set -euo pipefail
DIR="${HOME}/.claude/agents"

rewrite() { # $1=name $2=new-description
  local f="$DIR/$1.md"
  [ -f "$f" ] || { echo "MISSING $f" >&2; return 1; }
  if rg -q '^description:\s*Level ' "$f"; then
    sd '^description:.*$' "description: $2" "$f"
    echo "rewrote description: $1"
  else
    echo "skip (already imperative): $1"
  fi
}

rewrite ceo \
"Use this agent to orchestrate a multi-part build — scope the work, delegate to engineers, track outcomes; never writes code itself. Use PROACTIVELY for any task spanning two or more subsystems or needing parallel agents."

rewrite backend-engineer \
"Use this agent to implement production server-side code — APIs, handlers, services, database access — once a plan exists. Use for bounded backend implementation and refactors."

rewrite frontend-engineer \
"Use this agent to implement production UI — components, client-side code, state, and styling against a design system. Use when building or changing user-facing screens."

rewrite research-director \
"Use this agent BEFORE implementing in an unfamiliar domain — gathers evidence-based, URL-cited findings and never implements. Use PROACTIVELY when a library, API, version, or behavior must be confirmed from a current source."

rewrite nlu-intent-analyzer \
"Use this agent to classify a vague prompt and route it — single-turn parse, classify domain, pick the target specialist agent. Use when the right agent for a task is unclear."

rewrite aegis-guardian \
"Use this agent to verify a completed change — read-only lint, build, test, secrets and suppression audit plus bug-bounty enforcement. Use PROACTIVELY after any implementation, before declaring work done."

echo "Done: 6 passive descriptions converted to action imperatives in $DIR."

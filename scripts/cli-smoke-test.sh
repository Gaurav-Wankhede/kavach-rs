#!/usr/bin/env bash
# Smoke-test every kavach CLI command (read-only / --help where mutating).
set -uo pipefail

PROJECT="${1:-nicole-carpenter}"
CWD="${2:-/Users/gauravwankhede/Freelance/Nicole Carpenter}"
cd "$CWD" || exit 1

PASS=0 FAIL=0
RESULTS=""

run() {
  local label="$1"; shift
  local out rc
  if out=$("$@" 2>&1); then
    PASS=$((PASS + 1))
    RESULTS+="PASS|$label\n"
  else
    rc=$?
    local snippet
    snippet=$(echo "$out" | tr '\n' ' ' | cut -c1-100)
    FAIL=$((FAIL + 1))
    RESULTS+="FAIL|$label|exit=$rc|$snippet\n"
  fi
}

run_help() { run "$1" "${@:2}" --help; }

echo "=== TOP / HARNESS ==="
run "kavach --version" kavach --version
run "kavach status" kavach status
run_help "kavach --help" kavach
run "kavach context" kavach context --project "$PROJECT" --limit 2
run "kavach phase status" kavach phase status
run "kavach loop status" kavach loop status
run "kavach pipeline status" kavach pipeline status --project "$PROJECT"
run_help "kavach pipeline plan" kavach pipeline plan
run "kavach mistake stats" kavach mistake stats --project "$PROJECT"
run "kavach mistake list" kavach mistake list --project "$PROJECT" --limit 3
run_help "kavach mistake inspect" kavach mistake inspect
run_help "kavach mistake clear" kavach mistake clear
run_help "kavach mistake clear-all" kavach mistake clear-all
run_help "kavach deploy" kavach deploy
run_help "kavach verify" kavach verify
run_help "kavach verify-frontend" kavach verify-frontend
run_help "kavach rpc" kavach rpc
run_help "kavach mcp" kavach mcp
run_help "kavach app" kavach app
run_help "kavach ask" kavach ask
run "kavach toolbelt list" kavach toolbelt list
run_help "kavach toolbelt install" kavach toolbelt install

echo "=== SESSION ==="
for s in init validate end compact resume land end-hook clear-test-locks; do
  run_help "kavach session $s" kavach session "$s"
done

echo "=== RULES ==="
for s in list check generate show; do run_help "kavach rules $s" kavach rules "$s"; done

echo "=== PHASE ==="
run_help "kavach phase advance" kavach phase advance
run_help "kavach phase set" kavach phase set
run_help "kavach phase iteration-start" kavach phase iteration-start
run "kavach phase iteration-list" kavach phase iteration-list
run_help "kavach phase tier-set" kavach phase tier-set
run_help "kavach phase spike-start" kavach phase spike-start
run_help "kavach phase spike-end" kavach phase spike-end

echo "=== LOOP / SPEC ==="
run_help "kavach loop start" kavach loop start
run_help "kavach loop stop" kavach loop stop
run_help "kavach spec auto-draft" kavach spec auto-draft

echo "=== BULK / GOAL / BG / TEAM ==="
run_help "kavach bulk start" kavach bulk start
run_help "kavach bulk status" kavach bulk status
run_help "kavach bulk close" kavach bulk close
run_help "kavach goal start" kavach goal start
run_help "kavach goal status" kavach goal status
run_help "kavach goal stop" kavach goal stop
run_help "kavach goal compile" kavach goal compile
run_help "kavach goal reconcile" kavach goal reconcile
run_help "kavach bg start" kavach bg start
run_help "kavach bg status" kavach bg status
run_help "kavach bg stop" kavach bg stop
run_help "kavach team dispatch" kavach team dispatch

echo "=== TODOS / TASKS / SECURITY ==="
run_help "kavach todos sync" kavach todos sync
run "kavach tasks audit" kavach tasks audit
for s in init scan process report; do run_help "kavach security $s" kavach security "$s"; done

echo "=== OVERSIZED / TAILWIND / RAG ==="
run_help "kavach oversized scan" kavach oversized scan
run_help "kavach tailwind-plus index" kavach tailwind-plus index
run_help "kavach rag build" kavach rag build
run_help "kavach rag list" kavach rag list
run_help "kavach rag enrich-skills" kavach rag enrich-skills
run_help "kavach rag refresh-if-stale" kavach rag refresh-if-stale
run_help "kavach rag enrich" kavach rag enrich
run_help "kavach rag query" kavach rag query
run_help "kavach rag apply" kavach rag apply
run_help "kavach rag pending" kavach rag pending

echo "=== GATES (info) ==="
for g in stop session-start pre-write post-write pre-tool post-tool intent \
  pre-compact session-end post-tool-failure subagent-start subagent-stop \
  permission notification; do
  run "kavach gates $g" kavach gates "$g"
done

echo "=== GATES (six-file, hook smoke) ==="
run "kavach gates pre-implementation --hook" \
  bash -c 'echo "{\"hook_event_name\":\"PreImplementation\",\"cwd\":\"'"$CWD"'\"}" | kavach gates pre-implementation --hook --vendor cursor >/dev/null'
run "kavach gates post-implementation --hook" \
  bash -c 'echo "{\"hook_event_name\":\"PostImplementation\",\"cwd\":\"'"$CWD"'\"}" | kavach gates post-implementation --hook --vendor cursor >/dev/null'
run "kavach gates six-file-intent --hook" \
  bash -c 'echo "{\"hook_event_name\":\"UserPromptSubmit\",\"prompt\":\"test\",\"cwd\":\"'"$CWD"'\"}" | kavach gates six-file-intent --hook --vendor cursor >/dev/null'

echo "=== DB READ-ONLY ==="
run "kavach db kanban" kavach db kanban --project "$PROJECT" --limit 2
run "kavach db kanban --json" kavach db kanban --project "$PROJECT" --limit 2 --json
run "kavach db list-projects" kavach db list-projects
run "kavach db find-project" kavach db find-project --path "$CWD"
run "kavach db tree" kavach db tree
run "kavach db list-parts" kavach db list-parts --project "$PROJECT"
run "kavach db query" kavach db query --project "$PROJECT" --category roadmap
run "kavach db search" kavach db search --project "$PROJECT" --contains ironcore --limit 3
run "kavach db get" kavach db get --project "$PROJECT" --category roadmap --key roadmap.unit.platform.ironcore
run "kavach db graph-query" kavach db graph-query --limit 5
# RPC-only concept/bridge commands — run last after daemon is warm; retry once.
run_rpc_concept() {
  local label="$1"; shift
  if "$@" >/dev/null 2>&1; then
    PASS=$((PASS + 1)); RESULTS+="PASS|$label\n"
  elif sleep 3 && "$@" >/dev/null 2>&1; then
    PASS=$((PASS + 1)); RESULTS+="PASS|$label (retry)\n"
  else
    local out rc
    out=$("$@" 2>&1); rc=$?
    FAIL=$((FAIL + 1))
    RESULTS+="FAIL|$label|exit=$rc|$(echo "$out" | tr '\n' ' ' | cut -c1-100)\n"
  fi
}
run_rpc_concept "kavach db concept-list" kavach db concept-list --limit 5
run_rpc_concept "kavach db concept-search" kavach db concept-search --query auth --limit 3
run_rpc_concept "kavach db bridge-concepts-for" kavach db bridge-concepts-for --project "$PROJECT"
run_help "kavach db register" kavach db register
run_help "kavach db write" kavach db write
run_help "kavach db status-update" kavach db status-update
run_help "kavach db kanban-close" kavach db kanban-close
run_help "kavach db priority-set" kavach db priority-set
run_help "kavach db lane-set" kavach db lane-set
run_help "kavach db sync" kavach db sync
run_help "kavach db expire" kavach db expire
run_help "kavach db event" kavach db event
run_help "kavach db rotate" kavach db rotate
run_help "kavach db archive" kavach db archive
run_help "kavach db populate-graph" kavach db populate-graph
run_help "kavach db backfill-relationships" kavach db backfill-relationships
run_help "kavach db delete" kavach db delete
run_help "kavach db wipe-project" kavach db wipe-project
run_help "kavach db concept-add" kavach db concept-add
run_help "kavach db concept-link" kavach db concept-link
run_help "kavach db concept-delete" kavach db concept-delete
run_help "kavach db bridge-create" kavach db bridge-create
run_help "kavach db bridge-projects-for" kavach db bridge-projects-for
run_help "kavach db mistake-hit-count" kavach db mistake-hit-count
run_help "kavach db pg-introspect" kavach db pg-introspect
run_help "kavach db pg-isolation" kavach db pg-isolation
run_help "kavach db pg-er" kavach db pg-er
run_help "kavach db pg-drift" kavach db pg-drift
run_help "kavach db register-part" kavach db register-part
run_help "kavach db find-part" kavach db find-part
run_help "kavach db set-parent" kavach db set-parent

echo ""
echo "=== SUMMARY: PASS=$PASS FAIL=$FAIL TOTAL=$((PASS+FAIL)) ==="
printf "$RESULTS" | sort
exit $(( FAIL > 0 ? 1 : 0 ))

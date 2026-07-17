#!/usr/bin/env bash
# kimi-exec-loop.sh — orchestrator/worker loop:
#   planner (K3, or `kavach db next-prompt` auto-author) stores exec_prompts on
#   roadmap cards via `kavach db write --exec-prompt`; this loop serves each top
#   todo card's exec_prompt to a cheap Kimi Code worker (`kimi -p -m $MODEL`)
#   and tracks card status in the kavach db.
#
# Usage:
#   scripts/kimi-exec-loop.sh <project-slug> [max-tasks]
# Env:
#   KIMI_EXEC_MODEL  worker model alias (default: kimi-code/kimi-for-coding)
#
# Exit: 0 when the board is drained or max-tasks reached; 1 on worker failure
# (the failed card is reset to `todo` so a later run can retry it).
set -euo pipefail

PROJECT="${1:?usage: kimi-exec-loop.sh <project-slug> [max-tasks]}"
MAX="${2:-10}"
MODEL="${KIMI_EXEC_MODEL:-kimi-code/kimi-for-coding}"

command -v kavach >/dev/null || { echo "error: kavach not on PATH" >&2; exit 1; }
command -v kimi   >/dev/null || { echo "error: kimi CLI not on PATH" >&2; exit 1; }
command -v jq     >/dev/null || { echo "error: jq not on PATH" >&2; exit 1; }

for ((i = 1; i <= MAX; i++)); do
    KEY="$(kavach db kanban --project "$PROJECT" --status todo --json \
        | jq -r '.items[0].key // empty')"
    if [[ -z "$KEY" ]]; then
        echo "board drained: no todo cards left for $PROJECT"
        exit 0
    fi

    PROMPT="$(kavach db next-prompt --project "$PROJECT")" || {
        echo "error: no servable exec_prompt for top card $KEY" >&2
        exit 1
    }

    echo "[$i/$MAX] $KEY -> kimi -p -m $MODEL"
    kavach db status-update --project "$PROJECT" --category roadmap \
        --key "$KEY" --status in_progress

    if kimi -p -m "$MODEL" "$PROMPT"; then
        kavach db status-update --project "$PROJECT" --category roadmap \
            --key "$KEY" --status done
        echo "[$i/$MAX] $KEY done"
    else
        rc=$?
        kavach db status-update --project "$PROJECT" --category roadmap \
            --key "$KEY" --status todo
        echo "error: worker failed on $KEY (exit $rc); card reset to todo" >&2
        exit 1
    fi
done

echo "reached max-tasks=$MAX; rerun to continue"

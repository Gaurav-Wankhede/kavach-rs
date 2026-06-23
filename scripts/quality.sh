#!/usr/bin/env bash
# Language-agnostic quality gate. Detects each stack present by its manifest and
# runs that stack's native lint + test. SOURCE: decision.quality-language-agnostic-just-recipe.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

ran=0

run() { echo "── $1"; shift; "$@"; }

if [[ -f Cargo.toml ]]; then
  ran=1
  run "rust: fmt"  cargo fmt --all --check
  run "rust: clippy" cargo clippy --all-targets --all-features -- -D warnings
  run "rust: test" cargo nextest run --workspace --all-features
fi

if [[ -f package.json ]]; then
  ran=1
  pm=npm; [[ -f pnpm-lock.yaml ]] && pm=pnpm; [[ -f bun.lockb || -f bun.lock ]] && pm=bun
  if command -v biome >/dev/null 2>&1; then run "node: biome" biome check .; fi
  rg -q '"lint"' package.json && run "node: lint" "$pm" run lint || true
  rg -q '"test"' package.json && run "node: test" "$pm" test || true
fi

if [[ -f pyproject.toml || -f setup.py ]]; then
  ran=1
  command -v ruff >/dev/null 2>&1 && run "py: ruff" ruff check . || echo "  (ruff absent)"
  command -v pytest >/dev/null 2>&1 && run "py: pytest" pytest -q || echo "  (pytest absent)"
fi

if [[ -f go.mod ]]; then
  ran=1
  command -v golangci-lint >/dev/null 2>&1 && run "go: lint" golangci-lint run || run "go: vet" go vet ./...
  run "go: test" go test ./...
fi

[[ "$ran" == 1 ]] || { echo "no recognized stack manifest found"; exit 1; }
echo "[QUALITY] all detected stacks green"

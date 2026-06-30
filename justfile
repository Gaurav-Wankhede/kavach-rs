# justfile — engineering harness entry points for kavach-rs
# SOURCE: May 2026 Rust harness reference doc §9; cargo-deny + cargo-nextest
# canon. Operate via `just <recipe>`; `just --list` shows all.
#
# Tools required: just, cargo-nextest, cargo-deny, cargo-audit, cargo-machete,
# cargo-llvm-cov, bacon, watchexec, hyperfine. Bootstrap on a fresh machine via
# `cargo-binstall` (prebuilt binaries, ~10s vs minutes for source builds).
#
# Install everything missing:
#   brew install cargo-binstall                                # macOS bootstrap
#   cargo binstall cargo-nextest cargo-deny cargo-audit \
#                  cargo-machete cargo-llvm-cov bacon \
#                  watchexec-cli hyperfine

set dotenv-load := true
set shell := ["bash", "-uc"]

# Show all recipes
default:
    @just --list

# Build + install the CLI to ~/.local/bin for THIS host (host-adaptive: Mac
# M-chip / Intel / Linux). Delegates to `kavach deploy`, which on macOS does the
# fresh-inode + xattr-clear + ad-hoc codesign + exec-witness that prevents the
# exit-137 amfid SIGKILL, then restarts the RPC daemon onto the new binary.
install *ARGS:
    cargo run --release -p kavach-cli -- deploy {{ARGS}}

# Build + install the GUI app bundle (KavachApp.app + .dmg) with the CLI embedded
# as a sidecar, codesign --deep, install to /Applications, and symlink the
# terminal CLI into the bundle so CLI + GUI share one signed binary. macOS only;
# needs the `dx` (Dioxus 0.7) CLI.
bundle *ARGS:
    cargo run --release -p kavach-cli -- deploy --bundle {{ARGS}}

# ─────────────────────────────────────────────────────────────
# inner dev loop
# ─────────────────────────────────────────────────────────────

# Background TUI: re-runs cargo clippy on file change.
watch:
    bacon clippy

# Watch any file, run cargo check on change (lower-cost than clippy).
watch-check:
    watchexec -e rs -- cargo check --workspace

# ─────────────────────────────────────────────────────────────
# quality gates (also run by CI / `kavach deploy`)
# ─────────────────────────────────────────────────────────────

# Format check (denies unformatted code).
fmt:
    cargo fmt --all -- --check

# Apply formatting (mutating; for the dev loop, not CI).
fmt-fix:
    cargo fmt --all

# Lint with -D warnings (all warnings become errors).
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Process-isolated test run — each #[test] is its own parallel process.
# Tuning lives in .config/nextest.toml (timeouts, serial-env group).
test:
    cargo nextest run --profile ci --workspace --all-features
    cargo test --doc --workspace

# Test only one crate (workflow gate during a focused iteration).
test-one CRATE:
    cargo nextest run -p {{CRATE}} --all-features

# Re-run a single crate's tests on file change — inner-loop TDD.
watch-test CRATE:
    watchexec -e rs -- cargo nextest run -p {{CRATE}} --all-features

# Coverage with branch + line metrics, lcov + html output.
coverage:
    cargo llvm-cov nextest --all-features --lcov --output-path lcov.info
    cargo llvm-cov report --html --output-dir target/llvm-cov

# ─────────────────────────────────────────────────────────────
# security gates (the doc §5 high-leverage layer)
# ─────────────────────────────────────────────────────────────

# Combined policy enforcement: advisories + bans + licenses + sources.
deny:
    cargo deny check

# RustSec CVE scan only (cargo-deny `advisories` wraps the same DB,
# this is kept for parity with the doc and for CI matrices that run
# cargo-audit independently — e.g. via the GitHub Action).
audit:
    cargo audit

# ─────────────────────────────────────────────────────────────
# hygiene
# ─────────────────────────────────────────────────────────────

# Detect unused workspace dependencies (no build required; very fast).
unused:
    cargo machete

# Mutation testing — the highest-signal coverage check: inserts bugs, reports
# which survive your tests (= untested logic). Slow; run per-release not per-push.
# SOURCE: https://mutants.rs/ — install: cargo binstall cargo-mutants.
mutants *ARGS:
    cargo mutants --test-tool=nextest {{ARGS}}

# Language-agnostic quality gate: detect the stack from on-disk manifests and run
# its native lint+test. Polyglot repos run each present stack. SOURCE:
# decision.quality-language-agnostic-just-recipe.
quality:
    bash scripts/quality.sh

# ─────────────────────────────────────────────────────────────
# aggregate CI / deploy targets
# ─────────────────────────────────────────────────────────────

# Optimized whole-workspace release build — the artifact guarantee CI enforces
# beside the gates (LTO / opt-level / codegen-units + release-only cfg that the
# dev-profile lint/test never exercise). Kept OUT of `just ci` so the inner dev
# loop stays fast; CI invokes it as its own labeled step.
release:
    cargo build --workspace --release

# Full pre-merge gate: format, lint, test, security policy, hygiene.
# Mirrors what `kavach deploy` runs (which also builds + installs +
# codesigns). Use `just ci` for fast pre-push validation; `kavach
# deploy` for the binary cycle.
ci: fmt lint test deny unused
    @echo "[CI] all harness gates green"

# Same as `ci` but skips the long-running test phase. Useful when you
# only changed deps / lints and want a fast policy re-check.
ci-fast: fmt lint deny unused
    @echo "[CI-FAST] format + lint + security + hygiene green (tests skipped)"

# ─────────────────────────────────────────────────────────────
# diagnostics / helpers (toolbelt §2 patterns)
# ─────────────────────────────────────────────────────────────

# Find every TODO/FIXME/XXX — useful before a release tag.
todos:
    rg --line-number --color=never 'TODO|FIXME|XXX' crates/

# Disk-usage report for build artifacts (catches target/ bloat).
bloat:
    dust -d 2 target/

# Lines-of-code by language.
loc:
    tokei

# Benchmark a single command (smoke for perf regressions).
bench CMD:
    hyperfine --warmup 3 --runs 20 "{{CMD}}"

# Wiring audit — list RPC verbs + gates that are defined but never called.
# Orphans are CANDIDATES (verify the call path); see decision.audit.sh-vs-llm.
audit-wiring:
    bash scripts/audit_wiring.sh

# Runtime audit — actually invoke each read-only CLI verb + each hook gate,
# classify EMPTY / ERROR / OK. Proves responses, not just source wiring.
audit-runtime:
    bash scripts/audit_runtime.sh

# Rename all tests.rs files to {stem}_test.rs and fix references.
rename-test-files:
    bash scripts/rename-test-files.sh

# Retire 7 confirmed-orphan RPC verbs (audit #1717). Re-runnable.
retire-orphans:
    bash scripts/retire-orphans.sh
    cargo fmt -p kavach-rpc

# Author the full SDLC nano-agent roster into GLOBAL ~/.claude/agents (idempotent).
author-sdlc-agents:
    bash scripts/author-sdlc-agents.sh

# Convert passive "Level N" global-agent descriptions to action imperatives (idempotent).
agentize-descriptions:
    bash scripts/agentize-descriptions.sh

# Validate every mermaid block in an HTML/MD file with mmdc BEFORE it ships (exit 1 on syntax error).
mermaid-check FILE:
    bash scripts/mermaid-check.sh {{FILE}}

# Retire the BLOCKED_BY: dependency alias for canonical DEPENDS_ON: across source (idempotent).
rename-blocked-by:
    bash scripts/rename-blocked-by.sh

# Migrate DB rows off the retired blocker-prefix alias to DEPENDS_ON (idempotent; needs the daemon).
migrate-blocked-by-db:
    bash scripts/migrate-blocked-by-db.sh

export const meta = {
  name: 'clippy-zero-engine',
  description: 'Fan out one fix-agent per kavach-engine/cli file to drive release clippy to 0, then verify',
  phases: [
    { title: 'Fix', detail: 'one forbid-safe fix-agent per file' },
    { title: 'Verify', detail: 'single workspace release-clippy barrier' },
  ],
}

// args = { "crates/.../foo.rs": ["error: ...", ...], ... }
const FILES = Object.entries(args || {})

const RULES = `
You are fixing rustc/clippy errors in ONE file of a Rust 1.95 edition-2024 workspace.
The workspace is forbid(unwrap_used, expect_used, panic, unsafe_code) and denies a large clippy set
(pedantic + nursery + many restriction lints, promoted to errors by \`-D warnings\`).

HARD RULES — never violate:
- NEVER change a function SIGNATURE (params, return type, by-ref vs by-value). Other crates AND gate_runner.rs call these.
  In particular: do NOT change a gate \`run(...) -> Result<(), EngineError>\` to \`-> ()\`, and do NOT change \`-> bool\`/\`-> Result\`
  to \`()\` to satisfy "this function's return value is unnecessary". Instead add
  #[expect(clippy::must_use_candidate, reason="...")] OR #[expect(clippy::unnecessary_wraps, reason="uniform gate dispatch")] on the fn.
- NEVER append \`.ok()\` to a call that returns \`HookAction\` or \`()\` — only Results have \`.ok()\`. A \`kavach_hook::exit_*(...)\`
  call is a statement (\`...;\`) or the fn's tail expression — leave it as-is, do NOT add \`.ok()\`.
- \`kavach_rpc::client::call\` takes TWO generics: \`call::<ParamsType, ReturnType>(method, params)\`. Never drop one.
- "used expect() on a Result"/"expect_used": expect IS FORBIDDEN (forbid lint) — you CANNOT #[expect] it (E0453).
  Replace \`x.expect("m")\` with \`x.unwrap_or_else(|_| <safe default>)\` or \`let Ok(v) = x else { return <default> };\`. Pick a
  semantically-safe default by reading the function (a failed parse/regex in a detector usually means "no signal" → false/empty/None).
- NEVER add #[allow(...)]. Use #[expect(<lint>, reason="...")] ONLY when the code is provably correct
  and the lint is a restriction/pedantic false-positive. A bare #[expect] that the lint does not fire =
  unfulfilled_lint_expectations error, so only expect lints you SEE in the error list for that exact site.
- NEVER use .unwrap()/.expect()/panic!/unsafe.

MECHANICAL FIX MAP (apply the matching one per error):
- "arithmetic operation that can potentially result in unexpected side-effects" → use .saturating_add/_sub/_mul,
  or .checked_*(...).unwrap_or(default). If operands are provably bounded literals, add
  #[expect(clippy::arithmetic_side_effects, reason="...")] on the statement.
- "integer division" → if denom provably non-zero, #[expect(clippy::integer_division, reason="...")]; else .checked_div(d).unwrap_or(0).
- "floating-point arithmetic detected" → #[expect(clippy::float_arithmetic, reason="...")] on the let/expr (no integer alternative).
- "this could be rewritten as \`let...else\`" → rewrite the if-let/match to let-else as clippy's help shows.
- "case-sensitive file extension comparison" → replace \`.ends_with(".ext")\` with
  \`std::path::Path::new(x).extension().is_some_and(|e| e.eq_ignore_ascii_case("ext"))\`, or for the lowercase
  intent keep .ends_with but operate on an already-lowercased string. Match the surrounding intent.
- "\`format!(..)\` appended to existing \`String\`" → \`use std::fmt::Write as _;\` then \`write!(s, ...).ok();\` /
  \`writeln!(s, ...).ok();\` (NOT push_str(&format!()), NOT let _ = ).
- "this function's return value is unnecessary" → change the fn return type from \`-> bool\`/\`-> ()\`-wrapper to \`()\`
  if it always returns the same value AND the callers ignore it; if callers use it, leave signature and
  add #[expect(clippy::unnecessary_wraps / ...)] — but PREFER not touching signature: if unsure, add the expect.
- "non-binding \`let\` on an expression with \`#[must_use]\` type" / "on a result of a #[must_use] function" →
  replace \`let _ = expr;\` with \`expr.ok();\` if Result, or \`drop(expr);\` only if NOT Copy; for fmt Results use \`.ok();\`.
- "use of \`eprintln!\`" → kavach-engine has NO tracing dependency and stderr IS the intended hook-log channel.
  Do NOT add tracing. Add \`#[expect(clippy::print_stderr, reason="hook engine has no tracing dep; stderr is the hook log channel")]\`
  on the enclosing fn (one attr covers all eprintln! in that fn). Keep the eprintln! calls unchanged.
- "use of \`println!\`" → if it writes hook stdout protocol output, add \`#[expect(clippy::print_stdout, reason="hook stdout protocol channel")]\` on the fn; keep the call.
- "indexing into a string may panic" / "slicing may panic" / "indexing may panic" → replace \`x[a..b]\`/\`x[i]\`
  with \`x.get(a..b)\`/\`x.get(i)\` + handle None (\`.map_or(default, ...)\` or \`let Some(..) = .. else { continue/return }\`).
- "item in documentation is missing backticks" → wrap the flagged identifier in \`backticks\` inside the doc comment.
- "use Option::map_or instead of an if let/else" → rewrite \`if let Some(x)=o { f(x) } else { d }\` as \`o.map_or(d, f)\`.
- "use Option::map_or_else instead of an if let/else" → \`o.map_or_else(|| d, f)\`.
- "assigning the result of \`ToOwned::to_owned()\`/\`Clone::clone()\` may be inefficient" → use \`x.clone_into(&mut target)\` per clippy help.
- "docs for function returning \`Result\` missing \`# Errors\` section" → add a \`/// # Errors\n/// Returns ... when ...\` doc block above the fn.
- "adding items after statements is confusing" → move the \`fn\`/\`const\`/\`struct\` item to the TOP of its enclosing block/scope.
- "shadows a previous, unrelated binding" → rename the inner binding.
- "these match arms have identical bodies" → merge with \`|\` patterns.
- "this could be a \`const fn\`" → add \`const\` to the fn.
- "exported structs should not be exhaustive" → add #[expect(clippy::exhaustive_structs, reason="constructed cross-crate; non_exhaustive => E0639")].
- "exported enums should not be exhaustive" → add #[expect(clippy::exhaustive_enums, reason="matched cross-crate; non_exhaustive => E0004")].
- "this function has too many lines" → add #[expect(clippy::too_many_lines, reason="linear dispatcher; splitting hurts locality")] on the fn.
- "casting ... may cause a loss of precision" → use \`f64::from(x)\` for widening, or \`#[expect(clippy::cast_precision_loss, reason="...")]\` if the cast is intentional+bounded.
- "use of \`File::read_to_string\`/\`read_to_end\`" → use \`std::fs::read_to_string(path)?\` / \`std::fs::read(path)?\`.
- "more than 3 bools in a struct" / "passed by reference but more efficient by value" / "redundant expression" /
  "unnecessary safety comment" / "wildcard matches only a single variant" → apply clippy's \`help:\` suggestion verbatim; if it
  would change a public signature, add the matching #[expect(<lint>, reason="...")] instead.

PROCESS: Read the whole file. For EACH error line given, locate the site and apply the matching fix.
Then output ONLY a one-line summary "fixed N errors in <file>". Do not run cargo.
`

phase('Fix')
await pipeline(
  FILES,
  ([file, errs]) =>
    agent(
      `${RULES}\n\nFILE: ${file}\nERRORS (${errs.length}):\n${errs.map((e, i) => `${i + 1}. ${e}`).join('\n')}`,
      { label: `fix:${file.split('/').pop()}`, phase: 'Fix' }
    )
)

phase('Verify')
const verdict = await agent(
  `Run: cargo clippy --release -p kavach-cli -p kavach-engine -- -D warnings 2>&1 | rg -c '^error'\n` +
    `Report the integer count and, if > 0, the first 30 \`error\` lines with file:line. Return JSON {count, sample}.`,
  { label: 'verify:release-clippy', phase: 'Verify', agentType: 'general-purpose', schema: {
    type: 'object',
    required: ['count'],
    properties: { count: { type: 'integer' }, sample: { type: 'array', items: { type: 'string' } } },
    additionalProperties: false,
  } }
)
return verdict

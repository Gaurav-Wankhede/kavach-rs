export const meta = {
  name: 'clippy-zero-rpc',
  description: 'Drive kavach-cli + kavach-engine deploy-gate clippy to zero across kavach-rpc/chain/rule-engine/rule-generator',
  phases: [
    { title: 'Fix', detail: 'one agent per file fixes all its clippy errors (forbid-safe, SPLIT-rule)' },
    { title: 'Verify', detail: 'deploy-gate clippy -D warnings + workspace nextest barrier' },
  ],
}

// args.files = array of file paths (relative to repo root); args.clippyLog = path to full clippy stderr dump.
const files = Array.isArray(args?.files) ? args.files : []
const clippyLog = args?.clippyLog ?? '/tmp/clippy_full.txt'

if (files.length === 0) {
  log('No files passed in args.files — nothing to do.')
  return { fixed: 0, files: [] }
}

const FIX_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  required: ['file', 'errorsBefore', 'edited', 'summary'],
  properties: {
    file: { type: 'string' },
    errorsBefore: { type: 'integer' },
    edited: { type: 'boolean' },
    summary: { type: 'string', description: 'one-line description of fixes applied' },
  },
}

const RULES = `
You are fixing ALL release-clippy errors (cargo clippy --release -- -D warnings) in ONE Rust file.
This is the kavach-rs workspace: Rust 1.95, edition 2024, with these FORBID lints (cannot be silenced by #[allow]/#[expect] — they ERROR with E0453):
  unwrap_used, expect_used, panic, unsafe_code.

FORBID-SAFE PATTERNS (mandatory):
- NEVER add .unwrap()/.expect()/panic!/unsafe. Use ? , match, or .unwrap_or_else(closure) / .map_or_else.
- exhaustive_structs / exhaustive_enums: apply the SPLIT-RULE:
    * If the type is DESERIALIZE-ONLY (a wire DTO never constructed in this workspace, only serde-built) -> add #[non_exhaustive].
    * If the type is CONSTRUCTED in-crate (handler builds the Result/Item) OR exhaustively MATCHED cross-crate -> add
      #[expect(clippy::exhaustive_structs, reason = "...")] (or exhaustive_enums). RPC *Result/*Item/*Params built by handlers => use #[expect].
      Reason: #[non_exhaustive] on a constructed type triggers E0639 at the construction site; exhaustive match cross-crate triggers E0004.
    * When unsure for an RPC method DTO, prefer #[expect(clippy::exhaustive_structs/enums, reason="constructed at RPC handler boundary")].
- missing # Errors doc: add a '/// # Errors' doc section to the pub fn returning Result describing when it returns Err.
- option_if_let_else / "could be rewritten as let...else": rewrite per clippy's suggested form.
- format!(..) appended to String (e.g. s += &format!(...) or s.push_str(&format!(...))): replace with write!(s, ...).ok() is FORBIDDEN
  (silent_io). Use a push_str chain: s.push_str(a); s.push_str(b); — infallible, no format! alloc.
- arithmetic_side_effects: replace + - * with .saturating_add/.saturating_sub/.saturating_mul (or checked_* + handling) on the flagged op.
- partial_pub_fields / "mixed usage of pub and non-pub fields": make the struct's fields consistent — usually make the private field pub(crate) is WRONG; match the dominant visibility (make all pub OR all private with accessors). Prefer the minimal change clippy's help suggests.
- pub(crate) inside private module / unreachable_pub: if clippy says "consider using pub" AND that then triggers unreachable_pub, keep pub(crate) and add #[expect(clippy::redundant_pub_crate, reason="crate-internal, module is private")].
- too_long_first_doc_paragraph: split the first doc line so the first paragraph is short, or rephrase.
- too_many_lines: add #[expect(clippy::too_many_lines, reason="...")] ONLY if the function is a single linear dispatcher; otherwise leave it (don't refactor logic).
- doc_markdown: wrap bare identifiers like StatusCode, PgPool in backticks within doc comments.
- needless/unused async: remove 'async' from a fn with no .await IF no caller .awaits it (check). If risky, add #[expect(clippy::unused_async, reason=...)].
- match arms identical bodies / item-after-statements / binding shadows: apply clippy's mechanical suggestion.

HARD CONSTRAINTS:
- Edit ONLY the one file you are assigned. Do NOT touch other files.
- Do NOT change ANY runtime behavior, function signature semantics, struct field set, enum variant set, or test logic. Lint-clean MUST equal behavior-identical.
- Read the file fully before editing. Use Read then Edit.
- Do NOT add #[allow(...)]; use #[expect(..., reason="...")] with a real reason when suppression is the correct fix per the rules above.
`

phase('Fix')
const results = await pipeline(
  files,
  (file) => agent(
    `${RULES}\n\nTARGET FILE: ${file}\n\n` +
    `Step 1: Read ${file} fully.\n` +
    `Step 2: Read the clippy errors for THIS file by running: rg -n -A6 "${file}" ${clippyLog} | head -200  (use Bash). Every error block whose --> path is ${file} is yours.\n` +
    `Step 3: Apply forbid-safe fixes for EVERY clippy error in ${file}.\n` +
    `Step 4: Return the structured result. Do NOT run cargo (the workflow verifies centrally).`,
    { label: `fix:${file.split('/').slice(-2).join('/')}`, phase: 'Fix', schema: FIX_SCHEMA, agentType: 'general-purpose' }
  )
)

const edited = results.filter(Boolean)
log(`Fix phase complete: ${edited.length}/${files.length} files processed.`)

phase('Verify')
const gate = await agent(
  `Run the deploy-gate clippy and report the result. Use Bash:\n` +
  `  cargo clippy --release -p kavach-cli -p kavach-engine -- -D warnings 2>&1 | rg -c "^error" || echo 0\n` +
  `Then if that number is > 0, also run:\n` +
  `  cargo clippy --release -p kavach-cli -p kavach-engine -- -D warnings 2>&1 | rg "^error" | sort | uniq -c | sort -rn | head -20\n` +
  `Report: the remaining error count (integer) and, if non-zero, the per-category breakdown with the file:line of each remaining site.`,
  { label: 'verify:deploy-gate-clippy', phase: 'Verify', agentType: 'general-purpose' }
)

return { filesProcessed: edited.length, fixSummaries: edited, gateReport: gate }

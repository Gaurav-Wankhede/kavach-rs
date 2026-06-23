#!/usr/bin/env python3
"""Validate every Mermaid block embedded in the kavach LLD artifacts.

Offline structural linter for Mermaid v11 flowchart/mindmap blocks. Catches the
defects that silently render blank: chained bidirectional edges, raw HTML entities
in labels, unbalanced quotes/brackets, and empty diagrams. Source for the rules:
mermaid.js.org/syntax/flowchart.html (one link per statement; quote troublesome
text; entity codes use #NN; not &lt;).

Usage:  python3 scripts/validate_mermaid.py
Exit 0 = all blocks valid; exit 1 = at least one defect (printed with file:line).
"""
from __future__ import annotations
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TARGETS = [
    ROOT / "docs/architecture/kavach-lld.html",
    ROOT / "crates/kavach-engine/src/gates/session_start/lld.rs",
]

# In lld.rs the diagram is a Rust string literal: lines end with \n\ continuations
# and inner quotes are \" — normalise those so the validator sees real Mermaid.
def unescape_rust(s: str) -> str:
    s = s.replace('\\n\\', "\n").replace('\\n"', "\n").replace('\\"', '"')
    return s

MERMAID_RE = re.compile(r"```mermaid\s*\n(.*?)```", re.DOTALL)
VALID_HEADERS = ("flowchart", "graph", "mindmap", "sequenceDiagram", "classDiagram")

def extract_blocks(text: str) -> list[str]:
    return [m.group(1) for m in MERMAID_RE.finditer(text)]

def check_block(block: str, src: str, idx: int) -> list[str]:
    errs: list[str] = []
    lines = [ln.rstrip() for ln in block.splitlines() if ln.strip()]
    if not lines:
        errs.append(f"{src} block#{idx}: EMPTY diagram")
        return errs
    header = lines[0].strip()
    if not header.startswith(VALID_HEADERS):
        errs.append(f"{src} block#{idx}: unknown diagram header {header!r}")
    for n, ln in enumerate(lines, 1):
        # 1) chained bidirectional edges: A <--> B <--> C (invalid in v11)
        if ln.count("<-->") >= 2:
            errs.append(f"{src} block#{idx} L{n}: chained '<-->' (split into one edge per statement): {ln.strip()}")
        # 2) raw HTML entities inside a Mermaid label render literally
        for ent in ("&lt;", "&gt;", "&amp;"):
            if ent in ln:
                errs.append(f"{src} block#{idx} L{n}: raw HTML entity {ent} in label (use plain text): {ln.strip()}")
        # 3) unbalanced double quotes on a node/edge line
        if ln.count('"') % 2 != 0:
            errs.append(f"{src} block#{idx} L{n}: odd number of '\"' (unbalanced label quote): {ln.strip()}")
        # 4) unbalanced square brackets (node shape) ignoring mindmap which has none
        if header.startswith(("flowchart", "graph")):
            if ln.count("[") != ln.count("]"):
                errs.append(f"{src} block#{idx} L{n}: unbalanced [ ]: {ln.strip()}")
            if ln.count("(") != ln.count(")"):
                errs.append(f"{src} block#{idx} L{n}: unbalanced ( ): {ln.strip()}")
    return errs

def main() -> int:
    all_errs: list[str] = []
    total_blocks = 0
    for path in TARGETS:
        if not path.exists():
            all_errs.append(f"MISSING target: {path}")
            continue
        text = path.read_text(encoding="utf-8")
        if path.suffix == ".rs":
            text = unescape_rust(text)
        blocks = extract_blocks(text)
        if not blocks:
            all_errs.append(f"{path.name}: no ```mermaid blocks found")
            continue
        for i, b in enumerate(blocks, 1):
            total_blocks += 1
            all_errs.extend(check_block(b, path.name, i))
    if all_errs:
        print("MERMAID VALIDATION FAILED:")
        for e in all_errs:
            print(f"  ✘ {e}")
        return 1
    print(f"MERMAID OK: {total_blocks} block(s) across {len(TARGETS)} file(s) — no syntax defects.")
    return 0

if __name__ == "__main__":
    sys.exit(main())

//! Split logic + precision output for `kavach grep`.
//!
//! A keyword hit is resolved to the declaration that encloses it, that
//! declaration is auto-classified (function / class / struct / schema / ...),
//! and only that declaration's bytes are ever printed — never a byte outside
//! it.
//!
//! Large declarations are NOT cut with positional head/tail windows, because
//! that silently truncates precision. Instead the block keeps:
//!
//!   * the declaration signature (always),
//!   * a match-anchored window around every hit (always),
//!   * the terminator (always),
//!
//! and collapses the rest into `... N lines elided (La-Lb) ...` which names the
//! exact line range so a follow-up read can fetch precisely those lines.
//!
//! `ast-grep-outline` supplies the classification for 14 languages; languages
//! with no bundled rules simply degrade to bare matched lines.

use ast_grep_config::GlobalRules;
use ast_grep_core::tree_sitter::LanguageExt;
use ast_grep_language::SupportLang;
use ast_grep_outline::DEFAULT_OUTLINE_RULES;
use ast_grep_outline::combined_extractor::CombinedExtractors;
use ast_grep_outline::extractor::parse_outline_rules;
use ast_grep_outline::model::OutlineEntry;
use ast_grep_outline::model::SymbolType;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;

/// Total lines a single declaration block may print before it is elided.
pub(crate) const SCOPE_LINE_BUDGET: usize = 60;
/// Lines kept on each side of a hit inside an elided block.
const HIT_CONTEXT: usize = 3;
/// Leading lines always kept (the signature / header of the declaration).
const SIGNATURE_LINES: usize = 6;
/// Trailing lines always kept (the terminator, e.g. the closing `}`).
const TERMINATOR_LINES: usize = 2;

/// Byte-offset -> 1-based line lookup in O(log n).
pub(crate) struct LineTable {
    /// Byte offset of the first character of every line.
    starts: Vec<usize>,
}

impl LineTable {
    pub(crate) fn new(content: &str) -> Self {
        let mut starts = vec![0usize];
        for (idx, byte) in content.bytes().enumerate() {
            if byte == b'\n' {
                starts.push(idx + 1);
            }
        }
        Self { starts }
    }

    /// 1-based line containing `offset`.
    pub(crate) fn line_of(&self, offset: usize) -> usize {
        match self.starts.binary_search(&offset) {
            Ok(exact) => exact + 1,
            Err(insert_at) => insert_at,
        }
    }

    /// 1-based byte column of `offset` within its line.
    pub(crate) fn col_of(&self, offset: usize) -> usize {
        let line = self.line_of(offset);
        offset - self.starts[line - 1] + 1
    }

    /// Byte offset of the first character of 1-based `line`.
    pub(crate) fn line_start(&self, line: usize) -> usize {
        self.starts
            .get(line.saturating_sub(1))
            .copied()
            .unwrap_or(0)
    }
}

/// One declaration extracted from the syntax tree.
#[derive(Debug)]
pub(crate) struct DeclSlice {
    pub start: usize,
    pub end: usize,
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
    /// `"function process_file"`, `"class Parser"`, `"struct GrepHit"`, ...
    pub label: String,
}

impl DeclSlice {
    fn contains(&self, offset: usize) -> bool {
        self.start <= offset && offset < self.end
    }

    /// Header rendered above the block, e.g. `function process_file  272:5..380:1`.
    fn header(&self) -> String {
        format!(
            "{}  {}:{}..{}:{}",
            self.label, self.start_line, self.start_col, self.end_line, self.end_col
        )
    }
}

/// Flat, nest-aware map of every declaration in one file.
#[derive(Default)]
pub(crate) struct DeclarationIndex {
    slices: Vec<DeclSlice>,
}

thread_local! {
    /// Compiled extractors hold ast-grep matchers and are not `Sync`;
    /// `kavach grep` walks files on one thread, so thread-local caching is both
    /// correct and sufficient. Deserialising `DEFAULT_OUTLINE_RULES` and
    /// compiling patterns per file would dominate runtime otherwise.
    static EXTRACTORS: RefCell<HashMap<SupportLang, Option<Rc<CombinedExtractors<SupportLang>>>>> =
        RefCell::new(HashMap::new());
}

fn compile_extractors(lang: SupportLang) -> Option<CombinedExtractors<SupportLang>> {
    let rules = parse_outline_rules::<SupportLang>(DEFAULT_OUTLINE_RULES).ok()?;
    let mine: Vec<_> = rules
        .into_iter()
        .filter(|rule| rule.common().language == lang)
        .collect();
    if mine.is_empty() {
        return None;
    }
    CombinedExtractors::try_from(mine, &GlobalRules::default()).ok()
}

fn extractors_for(lang: SupportLang) -> Option<Rc<CombinedExtractors<SupportLang>>> {
    EXTRACTORS.with(|cache| {
        if let Some(hit) = cache.borrow().get(&lang) {
            return hit.clone();
        }
        let built = compile_extractors(lang).map(Rc::new);
        cache.borrow_mut().insert(lang, built.clone());
        built
    })
}

/// Canonical lowercase label for an outline symbol type.
///
/// Mirrors LSP `DocumentSymbol.kind` so a hit is classified without the caller
/// having to know the raw tree-sitter node kind of the grammar that produced it.
pub(crate) fn symbol_type_label(kind: SymbolType) -> &'static str {
    match kind {
        SymbolType::File => "file",
        SymbolType::Module => "module",
        SymbolType::Namespace => "namespace",
        SymbolType::Package => "package",
        SymbolType::Class => "class",
        SymbolType::Struct => "struct",
        SymbolType::Enum => "enum",
        SymbolType::EnumMember => "enum-member",
        SymbolType::Interface => "interface",
        SymbolType::Function => "function",
        SymbolType::Method => "method",
        SymbolType::Constructor => "constructor",
        SymbolType::Property => "property",
        SymbolType::Field => "field",
        SymbolType::Constant => "const",
        SymbolType::Variable => "var",
        SymbolType::TypeParameter => "type-param",
        SymbolType::Event => "event",
        SymbolType::Operator => "operator",
        SymbolType::String => "string",
        SymbolType::Number => "number",
        SymbolType::Boolean => "boolean",
        SymbolType::Array => "array",
        SymbolType::Object => "object",
        SymbolType::Key => "key",
        SymbolType::Null => "null",
    }
}

fn entry_label(entry: &OutlineEntry<'_>) -> String {
    let kind = symbol_type_label(entry.symbol_type);
    let name = entry.name.trim();
    if name.is_empty() {
        kind.to_string()
    } else {
        format!("{} {}", kind, name)
    }
}

/// Markdown section spans.
///
/// `ast-grep-outline` reports Markdown headings as `atx_heading` nodes, but in
/// the tree-sitter grammar a heading covers *only its own line* — the body is a
/// sibling, never a child. So the outline cannot answer "which section is this
/// line in", and a prose hit would fall back to a bare line. Sections are
/// therefore derived from heading levels directly: a heading owns every line
/// until the next heading of the same or shallower level.
fn markdown_sections(content: &str, table: &LineTable) -> Vec<DeclSlice> {
    let lines: Vec<&str> = content.lines().collect();
    // (line_no, level, title)
    let mut headings: Vec<(usize, usize, String)> = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix('#') {
            let level = rest.chars().take_while(|c| *c == '#').count();
            if (1..=6).contains(&level)
                && rest[level..].starts_with([' ', '\t'])
            {
                headings.push((idx + 1, level, rest[level..].trim().to_string()));
            }
        }
    }
    if headings.is_empty() {
        return Vec::new();
    }

    let mut sections = Vec::new();
    for (i, (start_line, level, title)) in headings.iter().enumerate() {
        let (start_line, level) = (*start_line, *level);
        // The section runs to the next heading at the same or shallower level.
        let end_line = headings
            .iter()
            .skip(i + 1)
            .find(|(_, next_level, _)| *next_level <= level)
            .map_or(lines.len(), |(next, _, _)| next.saturating_sub(1));
        if end_line < start_line {
            continue;
        }
        let start = table.line_start(start_line);
        let end = table
            .line_start(end_line)
            .saturating_add(lines.get(end_line - 1).map_or(0, |l| l.len()));
        sections.push(DeclSlice {
            start,
            end,
            start_line,
            start_col: indent_col(lines[start_line - 1]),
            end_line,
            end_col: indent_col(lines[end_line - 1]),
            label: format!("section {}", title),
        });
    }
    sections
}

impl DeclarationIndex {
    /// Build the index for `content`. Languages with no bundled outline rules
    /// (and unparseable sources) yield an empty index, degrading the split pass
    /// to plain matched-line output instead of failing the search.
    pub(crate) fn build(content: &str, lang: SupportLang) -> Self {
        let mut slices = Vec::new();
        let table = LineTable::new(content);

        // Markdown needs its own section resolver; see `markdown_sections`.
        if lang == SupportLang::Markdown {
            return Self {
                slices: markdown_sections(content, &table),
            };
        }

        let Some(extractors) = extractors_for(lang) else {
            return Self { slices };
        };
        let grep = lang.ast_grep(content);
        let root = grep.root();

        for item in extractors.extract(root) {
            push_entry(&item.entry, &table, &mut slices);
            for member in &item.members {
                push_entry(&member.entry, &table, &mut slices);
            }
        }

        Self { slices }
    }

    #[cfg(test)]
    pub(crate) fn slices(&self) -> &[DeclSlice] {
        &self.slices
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.slices.is_empty()
    }

    /// The *deepest* declaration containing `offset` — a method body wins over
    /// its class, a class wins over the module file. This is what keeps output
    /// inside the scope the caller actually asked about.
    pub(crate) fn enclosing(&self, offset: usize) -> Option<&DeclSlice> {
        let mut best: Option<&DeclSlice> = None;
        for slice in &self.slices {
            if slice.contains(offset) {
                let tighter = match best {
                    Some(current) => slice.end - slice.start < current.end - current.start,
                    None => true,
                };
                if tighter {
                    best = Some(slice);
                }
            }
        }
        best
    }
}

fn push_entry(entry: &OutlineEntry<'_>, table: &LineTable, out: &mut Vec<DeclSlice>) {
    let start = entry.range.byte_offset.start;
    let end = entry.range.byte_offset.end;
    if end <= start {
        return;
    }
    out.push(DeclSlice {
        start,
        end,
        start_line: table.line_of(start),
        start_col: table.col_of(start),
        end_line: table.line_of(end.saturating_sub(1)),
        end_col: table.col_of(end.saturating_sub(1)),
        label: entry_label(entry),
    });
}

/// 1-based byte column of the first non-whitespace character on `line`.
fn indent_col(line: &str) -> usize {
    line.len() - line.trim_start().len() + 1
}

fn emit_line<W: Write>(out: &mut W, lines: &[&str], number: usize) -> std::io::Result<()> {
    let content = lines.get(number - 1).copied().unwrap_or("");
    writeln!(out, "    {}:{}  {}", number, indent_col(content), content)
}

fn emit_gap<W: Write>(
    out: &mut W,
    lines: &[&str],
    from: usize,
    to: usize,
) -> std::io::Result<()> {
    if from > to {
        return Ok(());
    }
    let first = indent_col(lines.get(from - 1).copied().unwrap_or(""));
    writeln!(
        out,
        "    ... {} lines elided ({}:{}..{}:{}) ...",
        to - from + 1,
        from,
        first,
        to,
        indent_col(lines.get(to - 1).copied().unwrap_or(""))
    )
}

/// Record `from..=to` into `keep`, clamped to the declaration's own bounds.
fn mark_range(lo: usize, hi: usize, from: usize, to: usize, keep: &mut Vec<usize>) {
    for n in from..=to {
        if n >= lo && n <= hi {
            keep.push(n);
        }
    }
}

/// Which lines of a declaration block survive the elision pass.
fn visible_lines(decl: &DeclSlice, hits: &[usize], budget: usize) -> Vec<usize> {
    let (lo, hi) = (decl.start_line, decl.end_line);
    let total = hi.saturating_sub(lo) + 1;
    if total <= budget {
        return (lo..=hi).collect();
    }

    let mut keep: Vec<usize> = Vec::new();

    // Signature and terminator are never elided.
    mark_range(
        lo,
        hi,
        lo,
        lo.saturating_add(SIGNATURE_LINES - 1),
        &mut keep,
    );
    mark_range(lo, hi, hi.saturating_sub(TERMINATOR_LINES - 1), hi, &mut keep);
    // A match-anchored window around every hit.
    for &hit in hits {
        mark_range(
            lo,
            hi,
            hit.saturating_sub(HIT_CONTEXT),
            hit.saturating_add(HIT_CONTEXT),
            &mut keep,
        );
    }

    keep.sort_unstable();
    keep.dedup();
    keep
}

/// Print one declaration block: header, then the surviving lines, with exact
/// `line:col` anchors and named elision gaps between them.
pub(crate) fn write_block<W: Write>(
    out: &mut W,
    lines: &[&str],
    decl: &DeclSlice,
    hits: &[usize],
    budget: usize,
) -> std::io::Result<()> {
    writeln!(out, "    {}", decl.header())?;
    let keep = visible_lines(decl, hits, budget);
    let mut cursor = decl.start_line;
    for &line in &keep {
        if line > cursor {
            emit_gap(out, lines, cursor, line - 1)?;
        }
        emit_line(out, lines, line)?;
        cursor = line + 1;
    }
    if cursor <= decl.end_line {
        emit_gap(out, lines, cursor, decl.end_line)?;
    }
    Ok(())
}

/// Print bare matched lines with `line:col` anchors.
///
/// Deliberately gap-free: a text file is ripgrep semantics, where non-matching
/// lines between hits are simply not printed.
pub(crate) fn write_plain<W: Write>(
    out: &mut W,
    lines: &[&str],
    hits: &[usize],
) -> std::io::Result<()> {
    for &line in hits {
        emit_line(out, lines, line)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUST: &str = r#"use std::fmt;

pub const LIMIT: usize = 3;

pub struct Parser {
    pub name: String,
}

impl Parser {
    pub fn parse(&self, input: &str) -> Option<u32> {
        if input.is_empty() {
            return None;
        }
        let parsed = input.len() as u32;
        Some(parsed)
    }
}

fn free_function(a: u32) -> u32 {
    let doubled = a * 2;
    doubled
}

fn wide_pipeline(alpha: u32, beta: u32) -> u32 {
    let step_one = alpha + 1;
    let step_two = step_one * 2;
    let step_three = step_two + beta;
    let step_four = step_three * 3;
    let step_five = step_four - 1;
    let step_six = step_five + alpha;
    let step_seven = step_six * beta;
    let step_eight = step_seven + LIMIT as u32;
    let step_nine = step_eight - beta;
    let step_ten = step_nine * alpha;
    let step_eleven = step_ten + beta;
    let step_twelve = step_eleven * 2;
    let step_thirteen = step_twelve - alpha;
    let step_fourteen = step_thirteen + beta;
    let step_fifteen = step_fourteen * alpha;
    let step_sixteen = step_fifteen + LIMIT as u32;
    let step_seventeen = step_sixteen - 1;
    let step_eighteen = step_seventeen * beta;
    let step_nineteen = step_eighteen + alpha;
    let step_twenty = step_nineteen * 3;
    let step_twentyone = step_twenty - beta;
    let step_twentytwo = step_twentyone + alpha;
    let step_twentythree = step_twentytwo * 2;
    step_twentythree
}
"#;

    fn render(src: &str, needle: &str, budget: usize) -> String {
        let lang = SupportLang::Rust;
        let index = DeclarationIndex::build(src, lang);
        let table = LineTable::new(src);
        let lines: Vec<&str> = src.lines().collect();
        let offset = src.find(needle).expect("needle must exist in fixture");
        let hit_line = table.line_of(offset);

        let mut buf: Vec<u8> = Vec::new();
        match index.enclosing(offset) {
            Some(decl) => {
                write_block(&mut buf, &lines, decl, &[hit_line], budget).unwrap();
            }
            None => write_plain(&mut buf, &lines, &[hit_line]).unwrap(),
        }
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn line_table_maps_offsets_to_line_and_col() {
        let table = LineTable::new("abc\nde\nf");
        assert_eq!(table.line_of(0), 1);
        assert_eq!(table.col_of(0), 1);
        assert_eq!(table.line_of(3), 1);
        assert_eq!(table.line_of(4), 2);
        assert_eq!(table.col_of(4), 1);
        assert_eq!(table.col_of(5), 2);
        assert_eq!(table.line_of(7), 3);
        assert_eq!(table.line_start(2), 4);
        assert_eq!(table.line_start(3), 7);
    }

    #[test]
    fn index_classifies_and_spans_rust_declarations() {
        let index = DeclarationIndex::build(RUST, SupportLang::Rust);
        let labels: Vec<&str> = index
            .slices()
            .iter()
            .map(|s| s.label.as_str())
            .collect();

        assert!(labels.contains(&"struct Parser"), "got {labels:?}");
        assert!(labels.contains(&"method parse"), "got {labels:?}");
        assert!(labels.contains(&"function free_function"), "got {labels:?}");
        assert!(labels.contains(&"function wide_pipeline"), "got {labels:?}");
    }

    #[test]
    fn enclosing_picks_the_deepest_declaration() {
        let index = DeclarationIndex::build(RUST, SupportLang::Rust);
        // Inside `Parser::parse` — the method must win over its `impl` block.
        let inside_method = RUST.find("input.len()").unwrap();
        assert_eq!(index.enclosing(inside_method).unwrap().label, "method parse");

        // A struct field is a member and therefore deeper than its struct.
        let inside_field = RUST.find("pub name: String").unwrap();
        assert_eq!(index.enclosing(inside_field).unwrap().label, "field name");

        // Inside `wide_pipeline` — must not resolve to any other function.
        let inside_pipeline = RUST.find("step_seven").unwrap();
        assert_eq!(
            index.enclosing(inside_pipeline).unwrap().label,
            "function wide_pipeline"
        );
    }

    #[test]
    fn enclosing_never_reports_a_declaration_outside_the_hit() {
        let index = DeclarationIndex::build(RUST, SupportLang::Rust);
        // A hit in `free_function` must not resolve to `parse`.
        let tail = RUST.find("doubled").unwrap();
        assert_eq!(index.enclosing(tail).unwrap().label, "function free_function");

        // A hit that belongs to no declaration at all must resolve to nothing,
        // so it stays a bare line instead of being widened into a neighbour.
        let orphan = "fn solo() {}\n// dangling\nfn other() {}\n";
        let orphan_index = DeclarationIndex::build(orphan, SupportLang::Rust);
        let comment = orphan.find("dangling").unwrap();
        assert!(
            orphan_index.enclosing(comment).is_none(),
            "a comment must not inherit a declaration it is not inside"
        );
    }

    #[test]
    fn small_declaration_prints_in_full_with_header() {
        let out = render(RUST, "doubled", SCOPE_LINE_BUDGET);
        assert!(out.starts_with("    function free_function"), "got:\n{out}");
        assert!(out.contains("fn free_function(a: u32) -> u32 {"), "got:\n{out}");
        assert!(out.trim_end().ends_with('}'), "got:\n{out}");
        assert!(!out.contains("elided"), "small block must not elide:\n{out}");
    }

    #[test]
    fn large_declaration_elides_but_keeps_signature_hit_and_terminator() {
        let out = render(RUST, "step_seven", 8);
        assert!(out.starts_with("    function wide_pipeline"), "got:\n{out}");
        // Signature survives.
        assert!(out.contains("fn wide_pipeline("), "got:\n{out}");
        // The hit line survives verbatim.
        assert!(out.contains("let step_seven = step_six * beta;"), "got:\n{out}");
        // Terminator survives.
        assert!(out.trim_end().ends_with('}'), "got:\n{out}");
        // And the elision names the exact range it dropped.
        assert!(out.contains("lines elided ("), "got:\n{out}");
    }

    #[test]
    fn elision_never_severs_the_hit_or_the_declaration_bounds() {
        let out = render(RUST, "step_seven", 8);
        let start = RUST
            .lines()
            .position(|l| l.contains("fn wide_pipeline"))
            .unwrap()
            + 1;
        let end = start + RUST.lines().skip(start - 1).take_while(|l| !l.starts_with('}')).count();
        for line in out.lines() {
            let Some(num) = line
                .trim_start()
                .split(':')
                .next()
                .and_then(|n| n.parse::<usize>().ok())
            else {
                continue;
            };
            assert!(
                (start..=end).contains(&num),
                "line {num} escaped function span {start}-{end}:\n{out}"
            );
        }
        // The dropped region is fully recoverable from the header alone.
        assert!(out.contains(&format!("{start}:")), "got:\n{out}");
    }

    #[test]
    fn symbol_type_labels_are_lowercase_and_distinct() {
        assert_eq!(symbol_type_label(SymbolType::Function), "function");
        assert_eq!(symbol_type_label(SymbolType::Struct), "struct");
        assert_eq!(symbol_type_label(SymbolType::Class), "class");
        assert_eq!(symbol_type_label(SymbolType::EnumMember), "enum-member");
        assert_eq!(symbol_type_label(SymbolType::Constant), "const");
    }

    #[test]
    fn language_without_outline_rules_degrades_to_empty_index() {
        // Elixir has a tree-sitter grammar but no bundled outline rules.
        let index = DeclarationIndex::build("fn main() {}\n", SupportLang::Elixir);
        assert!(index.is_empty());
    }

    #[test]
    fn plain_rendering_has_no_gap_markers() {
        let mut buf: Vec<u8> = Vec::new();
        write_plain(&mut buf, &["a", "b", "c"], &[1, 3]).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert_eq!(out, "    1:1  a\n    3:1  c\n");
        assert!(!out.contains("elided"));
    }

    const MARKDOWN: &str = r#"# Title

intro prose

## Alpha

alpha body one
alpha body two

### Alpha Deep

deep body

## Beta

beta body
"#;

    #[test]
    fn markdown_sections_span_heading_to_next_peer_heading() {
        let index = DeclarationIndex::build(MARKDOWN, SupportLang::Markdown);
        let offset = MARKDOWN.find("alpha body one").unwrap();
        let label = &index.enclosing(offset).unwrap().label;
        assert_eq!(label, "section Alpha");

        // A nested heading wins over its parent section.
        let deep = MARKDOWN.find("deep body").unwrap();
        assert_eq!(index.enclosing(deep).unwrap().label, "section Alpha Deep");

        // A peer heading starts a new section — no bleed across siblings.
        let beta = MARKDOWN.find("beta body").unwrap();
        assert_eq!(index.enclosing(beta).unwrap().label, "section Beta");

        // Prose before the first heading is not owned by anything.
        let intro = MARKDOWN.find("intro prose").unwrap();
        assert!(index.enclosing(intro).is_none());
    }

    #[test]
    fn markdown_sections_nest_but_never_bleed_across_siblings() {
        let index = DeclarationIndex::build(MARKDOWN, SupportLang::Markdown);
        let sections = index.slices();
        let by_label = |name: &str| {
            sections
                .iter()
                .find(|s| s.label == name)
                .unwrap_or_else(|| panic!("missing section {name}"))
        };

        for section in sections {
            assert!(section.start_line <= section.end_line);
            assert!(section.start < section.end);
        }

        // A parent heading fully contains its nested child.
        let alpha = by_label("section Alpha");
        let deep = by_label("section Alpha Deep");
        assert!(alpha.start <= deep.start && deep.end <= alpha.end);

        // Peer headings are disjoint — `Alpha` must not reach into `Beta`.
        let beta = by_label("section Beta");
        assert!(
            alpha.end_line < beta.start_line,
            "Alpha {:?} must stop before Beta {:?}",
            (alpha.start_line, alpha.end_line),
            (beta.start_line, beta.end_line)
        );
    }
}
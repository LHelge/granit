//! Wiki-link logic of the editor, kept free of GPUI so it can be tested
//! without a window: where code is, where wiki-links are, the unclosed `[[`
//! before the caret, completion candidates and their ranking, the edit that
//! accepts a completion, and heading-anchor lookup.
//!
//! Every offset is a byte offset into the note body, like the editor's.

use std::collections::HashSet;
use std::ops::Range;

use granit_core::cave::Cave;

/// Byte ranges of `text` that are code, where wiki-link syntax is literal
/// text: fenced code blocks (fence lines included; an unclosed fence runs to
/// the end), indented code blocks, and inline code spans, which may wrap
/// across the lines of one paragraph.
///
/// Approximates CommonMark: an indented block needs a blank line (or the
/// start) before it and is not recognised inside a list, where indentation
/// continues the item; a code span ends at the paragraph, which is cut at
/// blank lines, headings, fences and list items.
pub fn code_ranges(text: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    // (fence char, fence length, start offset of the opening line)
    let mut fence: Option<(u8, usize, usize)> = None;
    // Start of the indented code block being read.
    let mut indented: Option<usize> = None;
    // Start of the paragraph being read, for inline code spans.
    let mut paragraph: Option<usize> = None;
    let mut previous_blank = true;
    let mut in_list = false;
    let mut line_start = 0;

    let close_paragraph = |paragraph: &mut Option<usize>, end: usize, out: &mut Vec<_>| {
        if let Some(start) = paragraph.take() {
            inline_code_spans(&text[start..end], start, out);
        }
    };

    for line in text.split_inclusive('\n') {
        let line_end = line_start + line.len();
        let content = line.trim_end_matches(['\n', '\r']);
        let blank = content.trim().is_empty();

        if let Some((open_ch, open_len, start)) = fence {
            if let Some((ch, len, rest)) = fence_marker(content) {
                if ch == open_ch && len >= open_len && rest.trim().is_empty() {
                    ranges.push(start..line_end);
                    fence = None;
                }
            }
            line_start = line_end;
            previous_blank = false;
            continue;
        }

        let indent = indent_width(content);
        if let Some(start) = indented {
            if blank || indent >= 4 {
                line_start = line_end;
                continue;
            }
            ranges.push(start..line_start);
            indented = None;
        }
        if !blank && indent >= 4 && previous_blank && !in_list && paragraph.is_none() {
            indented = Some(line_start);
            line_start = line_end;
            continue;
        }

        if let Some((ch, len, _)) = fence_marker(content) {
            close_paragraph(&mut paragraph, line_start, &mut ranges);
            fence = Some((ch, len, line_start));
            line_start = line_end;
            previous_blank = false;
            continue;
        }

        if blank {
            close_paragraph(&mut paragraph, line_start, &mut ranges);
        } else {
            let trimmed = content.trim_start();
            let list_item = indent < 4 && is_list_item(trimmed);
            if indent < 4 {
                // An indented line, or one right below the list, continues
                // it; an unindented paragraph after a blank line ends it.
                in_list = list_item || (in_list && (indent > 0 || !previous_blank));
            }
            if list_item || is_atx_heading(trimmed) {
                // A new block: code spans do not reach back across it.
                close_paragraph(&mut paragraph, line_start, &mut ranges);
            }
            paragraph.get_or_insert(line_start);
        }
        previous_blank = blank;
        line_start = line_end;
    }
    if let Some((_, _, start)) = fence {
        ranges.push(start..text.len());
    }
    if let Some(start) = indented {
        ranges.push(start..text.len());
    }
    close_paragraph(&mut paragraph, text.len(), &mut ranges);
    ranges.sort_by_key(|range| range.start);
    ranges
}

/// Leading indentation in columns, a tab counting as four.
fn indent_width(line: &str) -> usize {
    let mut width = 0;
    for ch in line.chars() {
        match ch {
            ' ' => width += 1,
            '\t' => width += 4 - width % 4,
            _ => break,
        }
    }
    width
}

/// `- `, `* `, `+ `, `1. ` or `1) ` (or the marker alone).
fn is_list_item(trimmed: &str) -> bool {
    let rest = if let Some(rest) = trimmed.strip_prefix(['-', '*', '+']) {
        rest
    } else {
        let digits = trimmed.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 || digits > 9 {
            return false;
        }
        match trimmed[digits..].strip_prefix(['.', ')']) {
            Some(rest) => rest,
            None => return false,
        }
    };
    rest.is_empty() || rest.starts_with([' ', '\t'])
}

fn is_atx_heading(trimmed: &str) -> bool {
    let level = trimmed.bytes().take_while(|&b| b == b'#').count();
    (1..=6).contains(&level)
        && (trimmed.len() == level || trimmed[level..].starts_with([' ', '\t']))
}

/// A fence line: up to three spaces, then three or more backticks or
/// tildes. Returns the fence char, the run length and the rest of the line.
fn fence_marker(line: &str) -> Option<(u8, usize, &str)> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 {
        return None;
    }
    let rest = &line[indent..];
    let ch = *rest.as_bytes().first()?;
    if ch != b'`' && ch != b'~' {
        return None;
    }
    let len = rest.bytes().take_while(|&b| b == ch).count();
    (len >= 3).then(|| (ch, len, &rest[len..]))
}

/// Push the inline code spans of one paragraph: a run of N backticks up to
/// the next run of exactly N backticks. An unmatched run is literal text.
fn inline_code_spans(text: &str, start: usize, out: &mut Vec<Range<usize>>) {
    let b = text.as_bytes();
    let run_at = |i: usize| b[i..].iter().take_while(|&&c| c == b'`').count();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'`' {
            i += 1;
            continue;
        }
        let open = run_at(i);
        let mut j = i + open;
        let mut close = None;
        while j < b.len() {
            if b[j] == b'`' {
                let run = run_at(j);
                if run == open {
                    close = Some(j + run);
                    break;
                }
                j += run;
            } else {
                j += 1;
            }
        }
        match close {
            Some(end) => {
                out.push(start + i..start + end);
                i = end;
            }
            None => i += open,
        }
    }
}

/// Whether `offset` lies inside one of `code` (see [`code_ranges`]).
pub fn in_code(code: &[Range<usize>], offset: usize) -> bool {
    code.iter().any(|range| range.contains(&offset))
}

/// A `[[target]]` or `[[target|label]]` outside code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WikiLink {
    /// The whole link, brackets included.
    pub range: Range<usize>,
    /// The part before an optional `|label`, trimmed.
    pub target: String,
}

/// Every wiki-link in `text` outside code, in order. Matches the web
/// editor's `\[\[([^[\]\n]+)\]\]`: no brackets or newlines inside, and an
/// empty target is not a link.
pub fn wiki_links(text: &str) -> Vec<WikiLink> {
    let code = code_ranges(text);
    let b = text.as_bytes();
    let mut links = Vec::new();
    let mut i = 0;
    while i + 1 < b.len() {
        if !(b[i] == b'[' && b[i + 1] == b'[') {
            i += 1;
            continue;
        }
        let inner_start = i + 2;
        let mut j = inner_start;
        while j < b.len() && !matches!(b[j], b'[' | b']' | b'\n') {
            j += 1;
        }
        if j > inner_start && b[j..].starts_with(b"]]") {
            let inner = &text[inner_start..j];
            let target = inner.split('|').next().unwrap_or_default().trim();
            if !target.is_empty() && !in_code(&code, i) {
                links.push(WikiLink {
                    range: i..j + 2,
                    target: target.to_string(),
                });
            }
            i = j + 2;
        } else {
            i += 1;
        }
    }
    links
}

/// The wiki-link covering byte `offset`, if any.
pub fn link_at(links: &[WikiLink], offset: usize) -> Option<&WikiLink> {
    links.iter().find(|link| link.range.contains(&offset))
}

/// The ranges of the links whose target does not resolve.
pub fn broken_link_ranges(
    links: &[WikiLink],
    is_resolved: impl Fn(&str) -> bool,
) -> Vec<Range<usize>> {
    links
        .iter()
        .filter(|link| !is_resolved(&link.target))
        .map(|link| link.range.clone())
        .collect()
}

/// An unclosed `[[` before the caret: the link being typed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkQuery {
    /// Byte offset of the first `[` of the `[[`.
    pub start: usize,
    /// What has been typed between the `[[` and the caret.
    pub query: String,
}

/// The link being typed at `cursor`: the last `[[` on the caret's line
/// before the caret, with no `]` or `|` between it and the caret, and not
/// inside code. `None` when there is none.
pub fn link_query(text: &str, cursor: usize) -> Option<LinkQuery> {
    let cursor = floor_char_boundary(text, cursor);
    let line_start = text[..cursor].rfind('\n').map_or(0, |i| i + 1);
    let before = &text[line_start..cursor];
    let open = before.rfind("[[")?;
    let typed = &before[open + 2..];
    if typed.contains([']', '|']) {
        return None;
    }
    let start = line_start + open;
    if in_code(&code_ranges(text), start) {
        return None;
    }
    Some(LinkQuery {
        start,
        query: typed.to_string(),
    })
}

/// The edit that accepts `label` for the link opened at `start`: replace
/// everything from after the `[[` to the caret (and a `]]` right after the
/// caret, so it is not doubled) with `label]]`. Returns the byte range to
/// replace and the replacement; the caret belongs at the range start plus
/// the replacement length.
pub fn completion_edit(
    text: &str,
    start: usize,
    cursor: usize,
    label: &str,
) -> (Range<usize>, String) {
    let cursor = floor_char_boundary(text, cursor);
    let from = start + 2;
    let to = if text[cursor..].starts_with("]]") {
        cursor + 2
    } else {
        cursor
    };
    (from..to, format!("{label}]]"))
}

fn floor_char_boundary(text: &str, offset: usize) -> usize {
    let mut offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

/// The byte offset in `text` of UTF-16 code unit `utf16` (what GPUI's input
/// handler reports), clamped to the text.
pub fn byte_offset_from_utf16(text: &str, utf16: usize) -> usize {
    let mut units = 0;
    for (offset, ch) in text.char_indices() {
        if units >= utf16 {
            return offset;
        }
        units += ch.len_utf16();
    }
    text.len()
}

/// The byte offset in `text` of the start of 0-based line `line`, clamped to
/// the last line.
pub fn line_offset(text: &str, line: usize) -> usize {
    if line == 0 {
        return 0;
    }
    text.match_indices('\n')
        .nth(line - 1)
        .map_or_else(|| text.rfind('\n').map_or(0, |at| at + 1), |(at, _)| at + 1)
}

/// The byte offset of the start of the ATX heading line that declares
/// `{#anchor}` (compared case-insensitively, like `Cave::resolve_link`),
/// skipping code.
pub fn anchor_heading_offset(text: &str, anchor: &str) -> Option<usize> {
    let want = anchor.to_lowercase();
    let code = code_ranges(text);
    let mut line_start = 0;
    for line in text.split_inclusive('\n') {
        let start = line_start;
        line_start += line.len();
        if in_code(&code, start) {
            continue;
        }
        if heading_anchor(line).is_some_and(|id| id.to_lowercase() == want) {
            return Some(start);
        }
    }
    None
}

/// The `{#id}` of an ATX heading line, if it has one.
fn heading_anchor(line: &str) -> Option<&str> {
    let line = line.trim_end();
    let trimmed = line.trim_start_matches(' ');
    if line.len() - trimmed.len() > 3 {
        return None;
    }
    let level = trimmed.bytes().take_while(|&b| b == b'#').count();
    if !(1..=6).contains(&level) || !trimmed[level..].starts_with([' ', '\t']) {
        return None;
    }
    let attrs = trimmed.strip_suffix('}')?;
    let open = attrs.rfind('{')?;
    attrs[open + 1..]
        .split_whitespace()
        .find_map(|token| token.strip_prefix('#'))
        .filter(|id| !id.is_empty())
}

/// What a completion entry links to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CandidateKind {
    Note,
    /// A heading anchor, with the note that declares it.
    Anchor {
        note: String,
    },
    /// A target other notes already link to but that has no note yet.
    Broken,
    /// Link the typed text, which matches nothing yet.
    Create,
}

/// One entry of the `[[` completion list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// The text inserted between `[[` and `]]`.
    pub label: String,
    pub kind: CandidateKind,
}

/// The cave's link targets: completion candidates and the set of names that
/// resolve. Rebuilt from the cave whenever it may have changed.
#[derive(Debug, Default)]
pub struct LinkIndex {
    candidates: Vec<Candidate>,
    /// Lowercased note slugs and anchor ids.
    resolved: HashSet<String>,
}

impl LinkIndex {
    /// `anchors` pairs each anchor id with the note declaring it.
    pub fn new(slugs: Vec<String>, anchors: Vec<(String, String)>, broken: Vec<String>) -> Self {
        let mut candidates: Vec<Candidate> = slugs
            .into_iter()
            .map(|label| Candidate {
                label,
                kind: CandidateKind::Note,
            })
            .collect();
        candidates.extend(anchors.into_iter().map(|(id, note)| Candidate {
            label: id,
            kind: CandidateKind::Anchor { note },
        }));
        let resolved: HashSet<String> = candidates
            .iter()
            .map(|candidate| candidate.label.to_lowercase())
            .collect();
        candidates.extend(
            broken
                .into_iter()
                .filter(|target| !resolved.contains(&target.to_lowercase()))
                .map(|label| Candidate {
                    label,
                    kind: CandidateKind::Broken,
                }),
        );
        Self {
            candidates,
            resolved,
        }
    }

    /// Build from the cave's in-memory indexes (no note is read).
    pub fn from_cave(cave: &Cave) -> Self {
        let slugs = cave.note_paths().keys().cloned().collect();
        let anchors = cave
            .list_anchors()
            .into_iter()
            .map(|id| {
                let note = cave
                    .resolve_link(&id)
                    .and_then(|href| href.rsplit_once('#').map(|(note, _)| note.to_string()))
                    .unwrap_or_default();
                (id, note)
            })
            .collect();
        Self::new(slugs, anchors, cave.list_broken_links())
    }

    /// Whether `target` resolves to a note or anchor (case-insensitive).
    pub fn is_resolved(&self, target: &str) -> bool {
        self.resolved.contains(&target.to_lowercase())
    }

    /// The completion list for `query`, best first, at most `limit` long.
    pub fn complete(&self, query: &str, limit: usize) -> Vec<Candidate> {
        rank(&self.candidates, query, limit)
    }
}

/// How well `label` matches `query` (both lowercased): lower is better,
/// `None` is no match. Exact, then prefix, then the start of a word, then
/// anywhere, then the query's characters in order.
fn match_score(label: &str, query: &str) -> Option<u8> {
    if query.is_empty() || label == query {
        return Some(0);
    }
    if label.starts_with(query) {
        return Some(1);
    }
    let word_start = label.match_indices(query).any(|(at, _)| {
        label[..at]
            .chars()
            .next_back()
            .is_some_and(|c| !c.is_alphanumeric())
    });
    if word_start {
        return Some(2);
    }
    if label.contains(query) {
        return Some(3);
    }
    let mut chars = label.chars();
    query.chars().all(|q| chars.any(|c| c == q)).then_some(4)
}

fn kind_rank(kind: &CandidateKind) -> u8 {
    match kind {
        CandidateKind::Note | CandidateKind::Anchor { .. } => 0,
        CandidateKind::Broken => 1,
        CandidateKind::Create => 2,
    }
}

/// Filter and order `candidates` for `query` (case-insensitive, trimmed):
/// by match quality, notes and anchors before broken targets, then shorter
/// and alphabetical. When the typed text matches no candidate exactly, a
/// [`CandidateKind::Create`] entry for it comes last.
pub fn rank(candidates: &[Candidate], query: &str, limit: usize) -> Vec<Candidate> {
    let typed = query.trim();
    let query = typed.to_lowercase();
    let mut scored: Vec<(u8, u8, usize, String, &Candidate)> = candidates
        .iter()
        .filter_map(|candidate| {
            let label = candidate.label.to_lowercase();
            let score = match_score(&label, &query)?;
            let len = if query.is_empty() { 0 } else { label.len() };
            Some((score, kind_rank(&candidate.kind), len, label, candidate))
        })
        .collect();
    let exact = scored.iter().any(|(_, _, _, label, _)| *label == query);
    scored.sort_by(|a, b| (a.0, a.1, a.2, &a.3).cmp(&(b.0, b.1, b.2, &b.3)));
    let mut out: Vec<Candidate> = scored
        .into_iter()
        .take(limit)
        .map(|(.., candidate)| candidate.clone())
        .collect();
    if !query.is_empty() && !exact {
        if out.len() == limit {
            out.pop();
        }
        out.push(Candidate {
            label: typed.to_string(),
            kind: CandidateKind::Create,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn targets(text: &str) -> Vec<String> {
        wiki_links(text).into_iter().map(|l| l.target).collect()
    }

    fn code(text: &str) -> Vec<&str> {
        code_ranges(text).iter().map(|r| &text[r.clone()]).collect()
    }

    fn note(label: &str) -> Candidate {
        Candidate {
            label: label.into(),
            kind: CandidateKind::Note,
        }
    }

    fn labels(candidates: &[Candidate]) -> Vec<&str> {
        candidates.iter().map(|c| c.label.as_str()).collect()
    }

    const WIKI_TARGETS: &str = "# Wiki targets\n\n\
        Plain: [[Test note]]\n\
        With label: [[Formatting tour|the tour]]\n\
        Non-ASCII slug: [[Räksmörgås och kaffe]]\n\
        Heading anchor: [[volvo]] and [[saab-anchor|Saab]]\n\
        In inline code, not a link: `[[Test note]]`\n\n\
        ```\nIn a fence, not a link: [[Test note]]\n```\n\n\
        ## Volvo {#volvo}\n\nText.\n\n## Saab {#saab-anchor}\n";

    #[test]
    fn code_ranges_cover_fences_and_inline_spans() {
        let text = "a `b` c\n```rust\nx\n```\nd ``e ` f`` g `h\n~~~~\ny\n~~~\nz\n";
        assert_eq!(
            code(text),
            ["`b`", "```rust\nx\n```\n", "``e ` f``", "~~~~\ny\n~~~\nz\n"]
        );
        // A closing fence needs the same char and at least the opening length.
        assert_eq!(code_ranges("````\na\n```\nb\n````\n").len(), 1);
    }

    #[test]
    fn indented_code_blocks_are_code() {
        let text = "Para\n\n    [[a]] code\n\tmore\n\nAfter [[b]]\n";
        assert_eq!(code(text), ["    [[a]] code\n\tmore\n\n"]);
        assert_eq!(targets(text), ["b"]);
        // Indentation right after a paragraph line continues it.
        assert!(code_ranges("Para\n    [[a]]\n").is_empty());
        // Inside a list, indentation continues the item.
        assert!(code_ranges("- item\n\n    [[a]]\n").is_empty());
        assert!(code_ranges("1. item\n   more\n\n    [[a]]\n").is_empty());
        assert!(code_ranges("- a\n\n  b\n\n    [[a]]\n").is_empty());
        // After the list has ended with a paragraph, it is code again.
        assert_eq!(code("- item\n\nPara\n\n    x\n"), ["    x\n"]);
        // At the very start of the note.
        assert_eq!(code("    x\ny\n"), ["    x\n"]);
    }

    #[test]
    fn inline_code_wraps_within_a_paragraph() {
        let text = "One `[[a]]\nstill code` and [[b]].\n\nNew `para\n\nnot` [[c]]\n";
        assert_eq!(code(text), ["`[[a]]\nstill code`"]);
        assert_eq!(targets(text), ["b", "c"]);
        // List items and headings are separate blocks.
        assert!(code_ranges("- `a\n- b`\n").is_empty());
        assert!(code_ranges("`a\n# b`\n").is_empty());
    }

    #[test]
    fn wiki_links_skip_code_and_take_the_target_before_the_label() {
        assert_eq!(
            targets(WIKI_TARGETS),
            [
                "Test note",
                "Formatting tour",
                "Räksmörgås och kaffe",
                "volvo",
                "saab-anchor"
            ]
        );
        assert_eq!(
            targets("[[ spaced | label ]] [[]] [[|x]] [[a\nb]]"),
            ["spaced"]
        );
        assert_eq!(targets("[[[a]] [[b]c]] ![[embed]]"), ["a", "embed"]);
    }

    #[test]
    fn link_ranges_are_byte_offsets_in_non_ascii_text() {
        let text = "Åäö [[Räksmörgås och kaffe|fika]] ö";
        let links = wiki_links(text);
        let range = links[0].range.clone();
        assert_eq!(&text[range.clone()], "[[Räksmörgås och kaffe|fika]]");
        assert_eq!(
            link_at(&links, range.start).unwrap().target,
            "Räksmörgås och kaffe"
        );
        assert!(link_at(&links, range.end).is_none());
        assert!(link_at(&links, 0).is_none());
    }

    #[test]
    fn broken_ranges_use_the_resolver() {
        let text = "[[a]] [[Missing|label]] `[[b]]`";
        let links = wiki_links(text);
        let broken = broken_link_ranges(&links, |t| t == "a" || t == "b");
        assert_eq!(
            broken.iter().map(|r| &text[r.clone()]).collect::<Vec<_>>(),
            ["[[Missing|label]]"]
        );
    }

    #[test]
    fn link_query_finds_the_unclosed_link_before_the_caret() {
        let text = "See [[Räk";
        assert_eq!(
            link_query(text, text.len()),
            Some(LinkQuery {
                start: 4,
                query: "Räk".into()
            })
        );
        assert_eq!(link_query("x [[", 4).unwrap().query, "");
        assert_eq!(
            link_query("[[Long note f", 13).unwrap().query,
            "Long note f"
        );
        assert_eq!(link_query("[[a]] b", 7), None);
        assert_eq!(link_query("[[a|b", 5), None);
        assert_eq!(link_query("[[a\nb", 5), None);
        assert_eq!(link_query("[[ab]]", 4).unwrap().query, "ab");
        // A mid-character caret snaps down instead of panicking.
        assert_eq!(link_query("[[å", 3).unwrap().query, "");
    }

    #[test]
    fn link_query_is_off_inside_code() {
        assert_eq!(link_query("`[[a` b`", 4), None);
        assert_eq!(link_query("```\n[[a", 7), None);
        let fenced = "```\nx\n```\n[[a";
        assert!(link_query(fenced, fenced.len()).is_some());
        // An unclosed backtick is not code.
        assert!(link_query("`[[a", 4).is_some());
        // Indented code and a code span wrapping onto the caret's line.
        let indented = "Para\n\n    [[a";
        assert_eq!(link_query(indented, indented.len()), None);
        let wrapped = "`x\n[[a` y";
        assert_eq!(link_query(wrapped, 6), None);
    }

    #[test]
    fn completion_edit_closes_the_link_without_doubling_brackets() {
        let text = "x [[Räk";
        let (range, insert) = completion_edit(text, 2, text.len(), "Räksmörgås och kaffe");
        let mut out = text.to_string();
        out.replace_range(range.clone(), &insert);
        assert_eq!(out, "x [[Räksmörgås och kaffe]]");
        assert_eq!(range.start + insert.len(), out.len(), "caret after ]]");

        let text = "[[Lo]] tail";
        let (range, insert) = completion_edit(text, 0, 4, "Long note for scrolling");
        let mut out = text.to_string();
        out.replace_range(range, &insert);
        assert_eq!(out, "[[Long note for scrolling]] tail");
    }

    #[test]
    fn utf16_offsets_map_to_bytes() {
        let text = "å😀[[x";
        assert_eq!(byte_offset_from_utf16(text, 0), 0);
        assert_eq!(byte_offset_from_utf16(text, 1), 2);
        assert_eq!(byte_offset_from_utf16(text, 3), 6);
        assert_eq!(&text[byte_offset_from_utf16(text, 4)..], "[x");
        assert_eq!(byte_offset_from_utf16(text, 99), text.len());
    }

    #[test]
    fn line_offsets_are_clamped() {
        let text = "a\nbb\nccc";
        assert_eq!(line_offset(text, 0), 0);
        assert_eq!(line_offset(text, 1), 2);
        assert_eq!(line_offset(text, 2), 5);
        assert_eq!(line_offset(text, 9), 5);
        assert_eq!(line_offset("", 3), 0);
        assert_eq!(line_offset("a\n", 1), 2);
    }

    #[test]
    fn anchor_headings_are_found_by_id() {
        let at = anchor_heading_offset(WIKI_TARGETS, "volvo").unwrap();
        assert!(WIKI_TARGETS[at..].starts_with("## Volvo {#volvo}"));
        let at = anchor_heading_offset(WIKI_TARGETS, "SAAB-anchor").unwrap();
        assert!(WIKI_TARGETS[at..].starts_with("## Saab"));
        assert_eq!(anchor_heading_offset(WIKI_TARGETS, "missing"), None);
        assert_eq!(anchor_heading_offset("```\n# A {#a}\n```\n", "a"), None);
        assert_eq!(anchor_heading_offset("#A {#a}\n", "a"), None);
        assert_eq!(anchor_heading_offset("x\n# Ö {#ö .cls}\n", "Ö"), Some(2));
    }

    #[test]
    fn ranking_prefers_exact_then_prefix_then_words() {
        let candidates = [
            note("Long note for scrolling"),
            note("Test note"),
            note("notes"),
            note("Räksmörgås och kaffe"),
            note("annotate"),
        ];
        assert_eq!(
            labels(&rank(&candidates, "note", 10)),
            [
                "notes",
                "Test note",
                "Long note for scrolling",
                "annotate",
                "note"
            ]
        );
        assert_eq!(
            labels(&rank(&candidates, "RÄK", 10)),
            ["Räksmörgås och kaffe", "RÄK"]
        );
        assert_eq!(
            labels(&rank(&candidates, "rsk", 10))[0],
            "Räksmörgås och kaffe"
        );
        assert_eq!(labels(&rank(&candidates, " test note ", 10)), ["Test note"]);
        assert_eq!(rank(&candidates, "", 10).len(), 5);
        assert_eq!(rank(&candidates, "", 2).len(), 2);
        let limited = rank(&candidates, "o", 2);
        assert_eq!(limited.len(), 2);
        assert_eq!(limited[1].kind, CandidateKind::Create);
    }

    #[test]
    fn index_ranks_resolving_targets_above_broken_ones() {
        let index = LinkIndex::new(
            vec!["Volvo history".into(), "Saab".into()],
            vec![("volvo".into(), "Wiki targets".into())],
            vec!["Volvo missing".into(), "saab".into()],
        );
        let found = index.complete("volvo", 10);
        assert_eq!(labels(&found), ["volvo", "Volvo history", "Volvo missing"]);
        assert_eq!(
            found[0].kind,
            CandidateKind::Anchor {
                note: "Wiki targets".into()
            }
        );
        assert_eq!(found[2].kind, CandidateKind::Broken);
        assert_eq!(index.complete("saab", 10).len(), 1);
        assert!(index.is_resolved("VOLVO"));
        assert!(index.is_resolved("saab"));
        assert!(!index.is_resolved("Volvo missing"));
    }
}

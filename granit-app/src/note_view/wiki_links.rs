//! Wiki-link logic of the editor, kept free of GPUI so it can be tested
//! without a window: the unclosed `[[` before the caret, completion
//! candidates and their ranking, and the edit that accepts a completion.
//!
//! Where links, code and heading anchors are comes from granit-core
//! ([`Markdown::link_spans`]), so the editor agrees with the renderer, the
//! backlinks and the broken-link list.
//!
//! Every offset is a byte offset into the note body, like the editor's.

use std::collections::HashSet;
use std::ops::Range;
use std::rc::Rc;

use granit_core::cave::Cave;
use granit_core::markdown::{LinkSpans, Markdown};

/// The [`LinkSpans`] of the last text asked for, so the several lookups of
/// one edit (broken links, completion) parse the body once.
#[derive(Default)]
pub struct SpanCache {
    text: String,
    spans: Rc<LinkSpans>,
}

impl SpanCache {
    /// The spans of `text`, parsed again only when it changed.
    pub fn get(&mut self, text: &str) -> Rc<LinkSpans> {
        if self.text != text {
            self.spans = Rc::new(Markdown::link_spans(text));
            self.text.clear();
            self.text.push_str(text);
        }
        self.spans.clone()
    }
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
/// inside code (`spans` are those of `text`). `None` when there is none.
pub fn link_query(text: &str, spans: &LinkSpans, cursor: usize) -> Option<LinkQuery> {
    let cursor = floor_char_boundary(text, cursor);
    let line_start = text[..cursor].rfind('\n').map_or(0, |i| i + 1);
    let before = &text[line_start..cursor];
    let open = before.rfind("[[")?;
    let typed = &before[open + 2..];
    if typed.contains([']', '|']) {
        return None;
    }
    let start = line_start + open;
    if spans.in_code(start) {
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

    /// Build from the cave's in-memory indexes (no note is read), in time
    /// linear in the notes and anchors.
    pub fn from_cave(cave: &Cave) -> Self {
        let slugs = cave.note_paths().keys().cloned().collect();
        let anchors = cave
            .anchor_notes()
            .map(|(id, note)| (id.to_string(), note.to_string()))
            .collect();
        Self::new(slugs, anchors, cave.list_broken_links())
    }

    /// Whether `target` resolves to a note or anchor (case-insensitive).
    pub fn is_resolved(&self, target: &str) -> bool {
        self.resolved.contains(&target.to_lowercase())
    }

    /// The ranges of the links in `spans` whose target does not resolve.
    pub fn broken_ranges(&self, spans: &LinkSpans) -> Vec<Range<usize>> {
        spans
            .links
            .iter()
            .filter(|link| !self.is_resolved(&link.target))
            .map(|link| link.range.clone())
            .collect()
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

    fn note(label: &str) -> Candidate {
        Candidate {
            label: label.into(),
            kind: CandidateKind::Note,
        }
    }

    fn labels(candidates: &[Candidate]) -> Vec<&str> {
        candidates.iter().map(|c| c.label.as_str()).collect()
    }

    fn query(text: &str, cursor: usize) -> Option<LinkQuery> {
        link_query(text, &Markdown::link_spans(text), cursor)
    }

    #[test]
    fn link_query_finds_the_unclosed_link_before_the_caret() {
        let text = "See [[Räk";
        assert_eq!(
            query(text, text.len()),
            Some(LinkQuery {
                start: 4,
                query: "Räk".into()
            })
        );
        assert_eq!(query("x [[", 4).unwrap().query, "");
        assert_eq!(query("[[Long note f", 13).unwrap().query, "Long note f");
        assert_eq!(query("[[a]] b", 7), None);
        assert_eq!(query("[[a|b", 5), None);
        assert_eq!(query("[[a\nb", 5), None);
        assert_eq!(query("[[ab]]", 4).unwrap().query, "ab");
        // A mid-character caret snaps down instead of panicking.
        assert_eq!(query("[[å", 3).unwrap().query, "");
    }

    #[test]
    fn link_query_is_off_inside_code() {
        assert_eq!(query("`[[a` b`", 4), None);
        assert_eq!(query("```\n[[a", 7), None);
        let fenced = "```\nx\n```\n[[a";
        assert!(query(fenced, fenced.len()).is_some());
        // An unclosed backtick is not code.
        assert!(query("`[[a", 4).is_some());
        // Indented code and a code span wrapping onto the caret's line.
        let indented = "Para\n\n    [[a";
        assert_eq!(query(indented, indented.len()), None);
        let wrapped = "`x\n[[a` y";
        assert_eq!(query(wrapped, 6), None);
        // Nor in raw HTML, where the renderer links nothing either.
        let html = "<div>\n[[a\n</div>\n";
        assert_eq!(query(html, 9), None);
    }

    #[test]
    fn the_span_cache_parses_again_only_after_a_change() {
        let mut cache = SpanCache::default();
        let first = cache.get("[[a]]");
        assert!(Rc::ptr_eq(&first, &cache.get("[[a]]")));
        let changed = cache.get("[[a]] [[b]]");
        assert_eq!(changed.links.len(), 2);
        assert_eq!(cache.get("").links.len(), 0);
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

    #[test]
    fn broken_ranges_skip_resolved_targets_and_code() {
        let index = LinkIndex::new(vec!["a".into(), "b".into()], Vec::new(), Vec::new());
        let text = "[[a]] [[Missing|label]] `[[nope]]` [[B]]";
        let broken = index.broken_ranges(&Markdown::link_spans(text));
        assert_eq!(
            broken.iter().map(|r| &text[r.clone()]).collect::<Vec<_>>(),
            ["[[Missing|label]]"]
        );
    }
}

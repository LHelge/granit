//! Rewriting `[[wiki-links]]` into Markdown links on a stream of chunks.
//!
//! GPUI Kit's Markdown view knows nothing about wiki-links, so the chat
//! rewrites `[[target]]` and `[[target|label]]` into `[label](<wiki:target>)`
//! before the text reaches the view; the view's link hook then resolves
//! `wiki:` targets against the cave (see `super::links`).
//!
//! Text inside code is left alone: inline code spans, fenced code blocks
//! (backticks or tildes) and indented code blocks. The rewriter works on a
//! stream, where a `[[`, a backtick run or a fence can be split across
//! chunks, and keeps the text it hands out append-only: a construct it
//! cannot decide yet is held back until the chunk that decides it arrives.
//! Only three things are held: a `[[` whose `]]` has not arrived (at most
//! the rest of its line), a backtick or tilde run that may still grow, and
//! a `[[` inside a code span that has not been closed yet (until the span
//! closes or its paragraph ends; an unclosed span is no span, and the link
//! is rewritten after all).
//!
//! The block structure is tracked line by line, close to CommonMark but not
//! exactly: an indented code block needs a blank line before it and is never
//! recognised inside a list (where four spaces of indentation are list
//! content), and only a blank line, a heading or a fence ends a paragraph
//! for code-span purposes.

/// The URL scheme rewritten wiki-links point at.
pub const WIKI_SCHEME: &str = "wiki:";

/// What the current line is, as far as it has been read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum LineKind {
    /// Only indentation and blockquote markers so far.
    #[default]
    Prefix,
    /// Ordinary text: wiki-links are rewritten.
    Text,
    /// Inside a fenced or indented code block.
    Code,
}

/// The scanner's state; cloned to rewind a held-back code span.
#[derive(Clone, Debug)]
struct Scan {
    /// The raw text of the current line read so far.
    line: String,
    kind: LineKind,
    /// Columns of indentation after any blockquote markers.
    indent: usize,
    /// A `>` was just read; the one space after it belongs to the marker.
    after_quote: bool,
    /// The open fence: its character and length.
    fence: Option<(char, usize)>,
    /// The fence was opened on the current line.
    fence_opened: bool,
    indented_code: bool,
    in_list: bool,
    /// The previous line was blank (or this is the first line).
    prev_blank: bool,
    /// The backtick count of the open code span.
    span: Option<usize>,
}

impl Default for Scan {
    fn default() -> Self {
        Self {
            line: String::new(),
            kind: LineKind::Prefix,
            indent: 0,
            after_quote: false,
            fence: None,
            fence_opened: false,
            indented_code: false,
            in_list: false,
            prev_blank: true,
            span: None,
        }
    }
}

/// Rewrites wiki-links on a stream; see the module docs.
#[derive(Default)]
pub struct WikiLinkRewriter {
    /// Text read but not handed out yet.
    pending: String,
    /// How much of `pending` has been scanned.
    cursor: usize,
    scan: Scan,
    /// While a `[[` inside an open code span is held back: the state at that
    /// `[[`, which sits at the start of `pending`.
    deferred: Option<Scan>,
    /// Link targets rewritten since the last [`Self::take_targets`].
    targets: Vec<String>,
}

/// What the scanner needs to do next.
enum Step {
    /// Consume this many bytes as they are.
    Keep(usize),
    /// Replace this many bytes with a link.
    Link { len: usize, replacement: String },
    /// Hold `[[` inside a code span until the span is decided.
    Defer,
    /// The held code span never closed: rescan it as text.
    Rewind,
    /// The buffer ends inside something undecided.
    Wait,
}

impl WikiLinkRewriter {
    /// Feed a chunk and get the text that is safe to append now.
    pub fn feed(&mut self, chunk: &str) -> String {
        self.pending.push_str(chunk);
        self.run(false)
    }

    /// The stream has ended: hand out everything still held, and close a
    /// fence left open, so text appended after it is not code. The
    /// rewriter starts over afterwards.
    pub fn finish(&mut self) -> String {
        let mut out = self.run(true);
        if let Some((ch, len)) = self.scan.fence {
            if !self.scan.line.is_empty() {
                out.push('\n');
            }
            out.extend(std::iter::repeat_n(ch, len));
            out.push('\n');
        }
        let targets = std::mem::take(&mut self.targets);
        *self = Self {
            targets,
            ..Self::default()
        };
        out
    }

    /// Rewrite a whole text at once.
    pub fn rewrite(text: &str) -> (String, Vec<String>) {
        let mut rewriter = Self::default();
        let mut out = rewriter.feed(text);
        out.push_str(&rewriter.finish());
        (out, rewriter.take_targets())
    }

    /// The targets of the links rewritten since the last call.
    pub fn take_targets(&mut self) -> Vec<String> {
        std::mem::take(&mut self.targets)
    }

    fn run(&mut self, at_end: bool) -> String {
        let mut out = String::new();
        loop {
            let step = match self.pending[self.cursor..].chars().next() {
                Some(c) => self.step(c, at_end),
                None if at_end && self.deferred.is_some() => Step::Rewind,
                None => break,
            };
            match step {
                Step::Keep(len) => self.consume(len),
                Step::Link { len, replacement } => {
                    out.push_str(&self.pending[..self.cursor]);
                    out.push_str(&replacement);
                    self.consume(len);
                    self.pending.drain(..self.cursor);
                    self.cursor = 0;
                }
                Step::Defer => {
                    out.push_str(&self.pending[..self.cursor]);
                    self.pending.drain(..self.cursor);
                    self.cursor = 0;
                    self.deferred = Some(self.scan.clone());
                    self.consume(2);
                }
                Step::Rewind => {
                    let mut scan = self.deferred.take().expect("rewind without a deferral");
                    scan.span = None;
                    self.scan = scan;
                    self.cursor = 0;
                }
                Step::Wait => break,
            }
            if self.deferred.is_none() && self.cursor > 0 {
                out.push_str(&self.pending[..self.cursor]);
                self.pending.drain(..self.cursor);
                self.cursor = 0;
            }
        }
        out
    }

    /// Read `len` bytes at the cursor into the current line.
    fn consume(&mut self, len: usize) {
        let text = &self.pending[self.cursor..self.cursor + len];
        // Only `end_line` consumes a newline, which starts a new line.
        if text != "\n" {
            self.scan.line.push_str(text);
        }
        self.cursor += len;
    }

    fn step(&mut self, c: char, at_end: bool) -> Step {
        if c == '\n' {
            return self.end_line();
        }
        if self.scan.kind == LineKind::Prefix {
            match c {
                ' ' => {
                    if !std::mem::take(&mut self.scan.after_quote) {
                        self.scan.indent += 1;
                    }
                    return Step::Keep(1);
                }
                '\t' => {
                    self.scan.after_quote = false;
                    self.scan.indent += 4 - self.scan.indent % 4;
                    return Step::Keep(1);
                }
                '>' if self.scan.fence.is_none() && !self.scan.indented_code => {
                    self.scan.indent = 0;
                    self.scan.after_quote = true;
                    return Step::Keep(1);
                }
                _ => {
                    if let Some(step) = self.start_content(c) {
                        return step;
                    }
                }
            }
        }
        if self.scan.kind == LineKind::Code {
            return Step::Keep(c.len_utf8());
        }
        // Ordinary text.
        let rest = &self.pending[self.cursor..];
        let at_content_start = self
            .scan
            .line
            .trim_start_matches([' ', '\t', '>'])
            .is_empty();
        match c {
            '\\' if self.scan.span.is_none() => match rest[1..].chars().next() {
                Some('\n') => Step::Keep(1),
                Some(next) => Step::Keep(1 + next.len_utf8()),
                None if at_end => Step::Keep(1),
                None => Step::Wait,
            },
            '`' | '~' => {
                let run = rest.chars().take_while(|ch| *ch == c).count();
                if run == rest.len() && !at_end {
                    return Step::Wait;
                }
                self.on_run(c, run, at_content_start)
            }
            '[' => match rest[1..].chars().next() {
                None if !at_end => Step::Wait,
                Some('[') if self.scan.span.is_some() => {
                    if self.deferred.is_some() {
                        Step::Keep(2)
                    } else {
                        Step::Defer
                    }
                }
                Some('[') => self.wiki_link(at_end),
                _ => Step::Keep(1),
            },
            _ => Step::Keep(c.len_utf8()),
        }
    }

    /// The first character after the line's prefix decides what the line
    /// is. Returns a step when the line ends a held code span.
    fn start_content(&mut self, c: char) -> Option<Step> {
        let scan = &mut self.scan;
        scan.after_quote = false;
        if scan.fence.is_some() || (scan.indented_code && scan.indent >= 4) {
            scan.kind = LineKind::Code;
        } else {
            scan.indented_code = false;
            if scan.indent >= 4 && scan.prev_blank && !scan.in_list {
                scan.indented_code = true;
                scan.kind = LineKind::Code;
            } else {
                scan.kind = LineKind::Text;
                if c == '#' {
                    // A heading ends the paragraph, and any code span in it.
                    if self.deferred.is_some() {
                        return Some(Step::Rewind);
                    }
                    scan.span = None;
                }
            }
        }
        None
    }

    /// A run of `run` backticks or tildes at the cursor.
    fn on_run(&mut self, c: char, run: usize, at_content_start: bool) -> Step {
        let scan = &mut self.scan;
        let fence_indent = scan.indent <= 3 || scan.in_list;
        if at_content_start && run >= 3 && fence_indent {
            if self.deferred.is_some() {
                // A fence ends the paragraph the held span was in.
                return Step::Rewind;
            }
            scan.span = None;
            scan.fence = Some((c, run));
            scan.fence_opened = true;
            scan.kind = LineKind::Code;
            return Step::Keep(run);
        }
        if c == '`' {
            match scan.span {
                Some(open) if open == run => {
                    scan.span = None;
                    // The held `[[` was inside a real code span.
                    self.deferred = None;
                }
                Some(_) => {}
                None => scan.span = Some(run),
            }
        }
        Step::Keep(run)
    }

    /// A `[[` at the cursor outside code.
    fn wiki_link(&mut self, at_end: bool) -> Step {
        let rest = &self.pending[self.cursor + 2..];
        let line_end = rest.find('\n');
        match rest.find("]]") {
            Some(close) if line_end.is_none_or(|nl| close < nl) => {
                let inner = &rest[..close];
                match link_markdown(inner) {
                    Some((replacement, target)) => {
                        self.targets.push(target);
                        Step::Link {
                            len: 2 + close + 2,
                            replacement,
                        }
                    }
                    None => Step::Keep(1),
                }
            }
            None if line_end.is_none() && !at_end => Step::Wait,
            _ => Step::Keep(1),
        }
    }

    /// The line is complete: update the block state from it.
    fn end_line(&mut self) -> Step {
        let line = std::mem::take(&mut self.scan.line);
        let content = line.trim_start_matches([' ', '\t', '>']);
        let blank = content.trim().is_empty();
        if blank && self.deferred.is_some() {
            // The paragraph ends with its code span still open.
            return Step::Rewind;
        }
        let scan = &mut self.scan;
        if let Some((ch, len)) = scan.fence {
            // The opening line looks like a closing one; it never closes.
            if !std::mem::take(&mut scan.fence_opened) && is_closing_fence(content, ch, len) {
                scan.fence = None;
            }
            scan.prev_blank = false;
        } else if blank {
            scan.prev_blank = true;
            scan.span = None;
        } else {
            if scan.kind == LineKind::Text {
                if is_list_item(content) {
                    scan.in_list = true;
                } else if scan.prev_blank && scan.indent == 0 {
                    scan.in_list = false;
                }
            }
            scan.prev_blank = false;
        }
        scan.kind = LineKind::Prefix;
        scan.indent = 0;
        scan.after_quote = false;
        // `consume` keeps the newline out of the next line.
        Step::Keep(1)
    }
}

/// The Markdown link for the inside of a `[[…]]`, and its target; `None`
/// when it is not a wiki-link.
fn link_markdown(inner: &str) -> Option<(String, String)> {
    if inner.contains(['[', ']', '<', '>', '`', '\n']) {
        return None;
    }
    let (target, label) = match inner.split_once('|') {
        Some((target, label)) => (target.trim(), label.trim()),
        None => (inner.trim(), inner.trim()),
    };
    if target.is_empty() {
        return None;
    }
    let label = if label.is_empty() { target } else { label };
    // The label is shown as written, not read as Markdown; in the
    // destination only a backslash could escape the closing `>`.
    Some((
        format!(
            "[{}](<{WIKI_SCHEME}{}>)",
            escape_punctuation(label),
            target.replace('\\', "\\\\")
        ),
        target.to_string(),
    ))
}

/// `text` with every ASCII punctuation character backslash-escaped.
fn escape_punctuation(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii_punctuation() {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Whether `content` (a line without its prefix) closes a fence of `len`
/// `ch`s.
fn is_closing_fence(content: &str, ch: char, len: usize) -> bool {
    let run = content.chars().take_while(|c| *c == ch).count();
    run >= len && content[run * ch.len_utf8()..].trim().is_empty()
}

/// Whether `content` (a line without its prefix) starts a list item (`-`,
/// `*`, `+`, `1.`, `1)`).
fn is_list_item(content: &str) -> bool {
    let marker_end = if content.starts_with(['-', '*', '+']) {
        1
    } else {
        let digits = content.chars().take_while(char::is_ascii_digit).count();
        if digits == 0 || digits > 9 || !content[digits..].starts_with(['.', ')']) {
            return false;
        }
        digits + 1
    };
    matches!(
        content[marker_end..].chars().next(),
        None | Some(' ' | '\t')
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rewrite `text` fed in chunks of `size` characters.
    fn in_chunks(text: &str, size: usize) -> String {
        let mut rewriter = WikiLinkRewriter::default();
        let chars: Vec<char> = text.chars().collect();
        let mut out = String::new();
        for chunk in chars.chunks(size) {
            out.push_str(&rewriter.feed(&chunk.iter().collect::<String>()));
        }
        out.push_str(&rewriter.finish());
        out
    }

    /// `text` rewrites to `expected` for every chunk size.
    fn check(text: &str, expected: &str) {
        for size in 1..=text.chars().count().max(1) {
            assert_eq!(in_chunks(text, size), expected, "chunk size {size}");
        }
    }

    #[test]
    fn rewrites_links_split_across_chunks() {
        check(
            "See [[Travel log]] and [[Packing list|the list]].",
            "See [Travel log](<wiki:Travel log>) and [the list](<wiki:Packing list>).",
        );
    }

    #[test]
    fn leaves_non_links_alone() {
        check(
            "[[open\nnext [a](b) [[]] end [",
            "[[open\nnext [a](b) [[]] end [",
        );
        check("\\[[x]] and [[a<b]]", "\\[[x]] and [[a<b]]");
    }

    #[test]
    fn skips_inline_code_spans() {
        check(
            "Run `[[code]]` and ``a ` [[b]]`` then [[c]].",
            "Run `[[code]]` and ``a ` [[b]]`` then [c](<wiki:c>).",
        );
    }

    #[test]
    fn an_unclosed_backtick_does_not_hide_a_link() {
        check(
            "A lone ` then [[x]].\n\nNext [[y]].",
            "A lone ` then [x](<wiki:x>).\n\nNext [y](<wiki:y>).",
        );
        check("Ends with ` and [[x]]", "Ends with ` and [x](<wiki:x>)");
    }

    #[test]
    fn skips_fenced_code_blocks() {
        check(
            "Before [[a]]\n```rust\nlet x = \"[[b]]\";\n```\nAfter [[c]]\n",
            "Before [a](<wiki:a>)\n```rust\nlet x = \"[[b]]\";\n```\nAfter [c](<wiki:c>)\n",
        );
        check(
            "~~~~\n[[a]]\n~~~\n[[b]]\n~~~~\n[[c]]",
            "~~~~\n[[a]]\n~~~\n[[b]]\n~~~~\n[c](<wiki:c>)",
        );
        check(
            "> ```\n> [[a]]\n> ```\n> [[b]]",
            "> ```\n> [[a]]\n> ```\n> [b](<wiki:b>)",
        );
    }

    #[test]
    fn skips_indented_code_but_not_nested_lists() {
        check(
            "Para [[a]]\n\n    [[code]]\n\nText [[b]]",
            "Para [a](<wiki:a>)\n\n    [[code]]\n\nText [b](<wiki:b>)",
        );
        check(
            "- item\n\n    more [[a]]\n    - nested [[b]]",
            "- item\n\n    more [a](<wiki:a>)\n    - nested [b](<wiki:b>)",
        );
        // Indentation that continues a paragraph is not code.
        check("Para\n    [[a]]", "Para\n    [a](<wiki:a>)");
    }

    #[test]
    fn finish_closes_an_open_fence() {
        let mut rewriter = WikiLinkRewriter::default();
        let mut out = rewriter.feed("```\ncode [[a]]");
        out.push_str(&rewriter.finish());
        assert_eq!(out, "```\ncode [[a]]\n```\n");
        // The rewriter starts over.
        assert_eq!(rewriter.feed("[[b]]") + &rewriter.finish(), "[b](<wiki:b>)");
    }

    #[test]
    fn labels_are_text_not_markdown() {
        check("[[*x*]]", "[\\*x\\*](<wiki:*x*>)");
        check("[[a\\]]", "[a\\\\](<wiki:a\\\\>)");
        check("[[t|_b_ & c]]", "[\\_b\\_ \\& c](<wiki:t>)");
    }

    #[test]
    fn reports_rewritten_targets() {
        let (out, targets) = WikiLinkRewriter::rewrite("[[a]] `[[b]]` [[ c | label ]]");
        assert_eq!(out, "[a](<wiki:a>) `[[b]]` [label](<wiki:c>)");
        assert_eq!(targets, vec!["a".to_string(), "c".to_string()]);
    }
}

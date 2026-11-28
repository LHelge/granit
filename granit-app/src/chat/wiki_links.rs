//! Rewriting `[[wiki-links]]` into Markdown links on a stream of chunks.
//!
//! GPUI Kit's Markdown view knows nothing about wiki-links, so the chat
//! rewrites `[[target]]` and `[[target|label]]` into `[label](<wiki:target>)`
//! before the text reaches the view; the view's link hook then resolves
//! `wiki:` targets against the cave (see `super::links`).
//!
//! What is a link and what is code (where `[[…]]` stays as written) is
//! granit-core's decision, the reader's: [`Markdown::stream_spans`] parses
//! the reply so far on every chunk. The text handed out is append-only, so
//! the rewriter hands out only the prefix the core calls settled (no later
//! chunk can change its links) and holds the rest: a `[[` whose `]]` has
//! not arrived, a link after a backtick or `<` that may still open a code
//! span or a tag, a code span whose closing run may still grow.

use granit_core::markdown::{Markdown, WikiLinkSpan};

/// The URL scheme rewritten wiki-links point at.
pub const WIKI_SCHEME: &str = "wiki:";

/// Rewrites wiki-links on a stream; see the module docs.
#[derive(Default)]
pub struct WikiLinkRewriter {
    /// The text fed since the stream started.
    text: String,
    /// How much of `text` has been handed out.
    emitted: usize,
    /// Link targets rewritten since the last [`Self::take_targets`].
    targets: Vec<String>,
}

impl WikiLinkRewriter {
    /// Feed a chunk and get the text that is safe to append now.
    pub fn feed(&mut self, chunk: &str) -> String {
        self.text.push_str(chunk);
        let stream = Markdown::stream_spans(&self.text);
        self.emit(&stream.spans.links, stream.settled)
    }

    /// The stream has ended: hand out everything still held, and close a
    /// fence left open, so text appended after it is not code. The
    /// rewriter starts over afterwards.
    pub fn finish(&mut self) -> String {
        let spans = Markdown::link_spans(&self.text);
        let mut out = self.emit(&spans.links, self.text.len());
        if let Some(closing) = Markdown::closing_fence(&self.text) {
            out.push_str(&closing);
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

    /// Hand out the text up to `until`, with its `links` rewritten.
    fn emit(&mut self, links: &[WikiLinkSpan], until: usize) -> String {
        // What was handed out stays handed out.
        let until = until.max(self.emitted);
        let mut out = String::new();
        let mut cursor = self.emitted;
        for link in links {
            if link.range.start < cursor || link.range.end > until {
                continue;
            }
            out.push_str(&self.text[cursor..link.range.start]);
            let (markdown, target) = link_markdown(link);
            out.push_str(&markdown);
            self.targets.push(target);
            cursor = link.range.end;
        }
        out.push_str(&self.text[cursor..until]);
        self.emitted = until;
        out
    }
}

/// The Markdown link for a wiki-link, and the target it points at.
fn link_markdown(link: &WikiLinkSpan) -> (String, String) {
    // A link destination in `<…>` holds no line break.
    let target = link.target.replace(['\r', '\n'], " ");
    let label = link
        .label
        .as_deref()
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .unwrap_or_else(|| target.trim());
    // The label is shown as written, not read as Markdown; in the
    // destination a backslash escapes the brackets and itself.
    let mut destination = String::with_capacity(target.len());
    for c in target.chars() {
        if matches!(c, '\\' | '<' | '>') {
            destination.push('\\');
        }
        destination.push(c);
    }
    (
        format!(
            "[{}](<{WIKI_SCHEME}{destination}>)",
            escape_punctuation(label)
        ),
        target,
    )
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
            "[[open\n\nnext [a](b) [[]] end [",
            "[[open\n\nnext [a](b) [[]] end [",
        );
        check("\\[[x]] and ![[embed]]", "\\[[x]] and ![[embed]]");
    }

    #[test]
    fn skips_inline_code_spans() {
        check(
            "Run `[[code]]` and ``a ` [[b]]`` then [[c]].",
            "Run `[[code]]` and ``a ` [[b]]`` then [c](<wiki:c>).",
        );
        // A closing run that grows closes nothing.
        check("`[[a]]`` x", "`[a](<wiki:a>)`` x");
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
    fn skips_indented_code_like_the_reader() {
        check(
            "Para [[a]]\n\n    [[code]]\n\nText [[b]]",
            "Para [a](<wiki:a>)\n\n    [[code]]\n\nText [b](<wiki:b>)",
        );
        check(
            "- item\n\n    more [[a]]\n    - nested [[b]]",
            "- item\n\n    more [a](<wiki:a>)\n    - nested [b](<wiki:b>)",
        );
        // Six spaces in a list item are four past its content: code.
        check(
            "- item\n\n      [[x]]\n\nAfter [[y]]",
            "- item\n\n      [[x]]\n\nAfter [y](<wiki:y>)",
        );
        // Indentation that continues a paragraph is not code.
        check("Para\n    [[a]]", "Para\n    [a](<wiki:a>)");
    }

    #[test]
    fn agrees_with_the_core_for_every_chunk_split() {
        let text = "Intro [[a]] `x\n- item\n\n      [[x]]\n\n```\n[[b]]\n```\n\
                    > [[c|label]] and `[[d]]`\n\n| [[e]] | f |\n|---|---|\n";
        let links: Vec<String> = Markdown::link_spans(text)
            .links
            .into_iter()
            .map(|link| link.target)
            .collect();
        for size in 1..=text.len() {
            let mut rewriter = WikiLinkRewriter::default();
            let chars: Vec<char> = text.chars().collect();
            for chunk in chars.chunks(size) {
                rewriter.feed(&chunk.iter().collect::<String>());
            }
            rewriter.finish();
            assert_eq!(rewriter.take_targets(), links, "chunk size {size}");
        }
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
        check("[[t|_b_ & c]]", "[\\_b\\_ \\& c](<wiki:t>)");
        check("[[a<b]]", "[a\\<b](<wiki:a\\<b>)");
    }

    #[test]
    fn reports_rewritten_targets() {
        let (out, targets) = WikiLinkRewriter::rewrite("[[a]] `[[b]]` [[c| label ]]");
        assert_eq!(out, "[a](<wiki:a>) `[[b]]` [label](<wiki:c>)");
        assert_eq!(targets, vec!["a".to_string(), "c".to_string()]);
    }
}

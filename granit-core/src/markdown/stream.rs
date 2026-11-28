//! Wiki-links of a text that is still growing, such as an agent reply
//! streamed in chunks.
//!
//! [`Markdown::link_spans`] says where the links and code of a text are, but
//! text appended later can change that for what came before it: a `[[` gets
//! its `]]`, a backtick finds its closing run, a code span's closing run
//! grows into a longer one, a `<` becomes an HTML tag, or a delimiter row
//! turns a paragraph's first line into a table header. [`StreamSpans`]
//! adds how much of the text is settled, so a consumer that hands the text
//! out append-only (rewriting the links on the way) never hands out a link
//! the finished text would not have, or misses one it would.
//!
//! The settled length is conservative: it may stop short of what is really
//! decided (a link then waits for a later chunk, or the end of the text),
//! never beyond it.

use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, LinkType, Parser, Tag, TagEnd};

use super::links::link_options;
use super::{LinkSpans, Markdown};

/// The wiki-links and code of a growing text, and how much of it is
/// settled.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StreamSpans {
    /// The text's links and code, as [`Markdown::link_spans`] finds them.
    pub spans: LinkSpans,
    /// The length of the prefix no appended text can change: every link of
    /// [`Self::spans`] that ends within it stays a link, and every `[[` in it
    /// that is no link stays text. Never inside a link.
    pub settled: usize,
}

/// A block whose inline content can hold a code span, while it is open in
/// the event stream.
struct Container {
    range: Range<usize>,
    /// Its first line could still become a table header (a paragraph or a
    /// list item's text; a table cannot start inside a paragraph).
    may_be_table: bool,
    /// It holds a backtick or `<` that opened nothing yet: a code span, an
    /// HTML tag or an autolink that later text may close.
    open_marker: bool,
}

impl Markdown<'_> {
    /// [`Markdown::link_spans`] of `text`, a text that is still growing, and
    /// how much of it is settled (see [`StreamSpans::settled`]).
    pub fn stream_spans(text: &str) -> StreamSpans {
        StreamSpans {
            spans: Markdown::link_spans(text),
            settled: settled_len(text),
        }
    }

    /// The line that closes a fenced code block left open at the end of
    /// `text`, so that a line appended after it is no longer code: the
    /// opening fence, indented like it (and inside the same blockquotes),
    /// on a line of its own. `None` when no fenced block is open, or when
    /// any appended line ends it anyway (a fence in a blockquote or list
    /// item ends with its container).
    pub fn closing_fence(text: &str) -> Option<String> {
        let block = Parser::new_ext(text, link_options())
            .into_offset_iter()
            .filter_map(|(event, range)| {
                matches!(
                    event,
                    Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(_)))
                )
                .then_some(range)
            })
            .last()
            .filter(|block| block.end >= text.len())?;
        // The block reaches the end; its last line may have closed it.
        let probe = format!("{text}\nx");
        if !Markdown::link_spans(&probe).in_code(probe.len() - 1) {
            return None;
        }
        let line_start = text[..block.start].rfind('\n').map_or(0, |nl| nl + 1);
        let fence_at = block.start + text[block.start..].len()
            - text[block.start..].trim_start_matches([' ', '\t']).len();
        let fence_char = text[fence_at..].chars().next()?;
        let fence_len = text[fence_at..]
            .chars()
            .take_while(|c| *c == fence_char)
            .count();
        let mut closing = String::new();
        if !text.ends_with('\n') {
            closing.push('\n');
        }
        // Blockquote markers and tabs stay, list markers become indentation.
        closing.extend(text[line_start..fence_at].chars().map(|c| match c {
            '>' | '\t' => c,
            _ => ' ',
        }));
        closing.extend(std::iter::repeat_n(fence_char, fence_len));
        closing.push('\n');
        Some(closing)
    }
}

/// See [`StreamSpans::settled`].
fn settled_len(text: &str) -> usize {
    let end = text.len();
    let mut settled = end;
    let mut containers: Vec<Container> = Vec::new();
    // A code or HTML block: the events inside it are skipped.
    let mut covered = 0..0;
    for (event, range) in Parser::new_ext(text, link_options()).into_offset_iter() {
        if covered.contains(&range.start) {
            continue;
        }
        // Whether the innermost container is still open at the end, so
        // what follows can still change its inline content.
        let live = containers.last().is_none_or(|c| c.range.end >= end);
        match event {
            Event::Start(Tag::CodeBlock(_) | Tag::HtmlBlock) => covered = range,
            Event::Start(
                tag @ (Tag::Paragraph | Tag::Heading { .. } | Tag::Item | Tag::TableCell),
            ) => containers.push(Container {
                range,
                may_be_table: matches!(tag, Tag::Paragraph | Tag::Item),
                open_marker: false,
            }),
            Event::End(
                TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::Item | TagEnd::TableCell,
            ) => {
                containers.pop();
            }
            Event::Text(_) if live => {
                let source = &text[range.clone()];
                if source.contains(['`', '<']) {
                    if let Some(container) = containers.last_mut() {
                        container.open_marker = true;
                    }
                }
                // A `[[` with no `]]` after it may still become a link, and
                // a `[` at the very end may still become a `[[`.
                for (ix, _) in source.match_indices('[') {
                    let at = range.start + ix;
                    let rest = &text[at..];
                    if rest == "[" || (rest.starts_with("[[") && !rest[2..].contains("]]")) {
                        settled = settled.min(at);
                    }
                }
            }
            Event::Start(Tag::Link {
                link_type: LinkType::WikiLink { has_pothole },
                ..
            }) => {
                let container = containers.last();
                let undecided = range.end >= end
                    || (live
                        && container.is_some_and(|c| {
                            c.open_marker
                                || (has_pothole
                                    && c.may_be_table
                                    && may_become_table_header(text, &c.range, range.start))
                        }));
                if undecided {
                    settled = settled.min(range.start);
                }
            }
            // Its closing run may still grow into one that closes nothing.
            Event::Code(_) if range.end >= end => settled = settled.min(range.start),
            _ => {}
        }
    }
    settled
}

/// Whether offset `at` lies on the first line of the block `container`,
/// whose second line is not complete yet: a delimiter row there would make
/// the first line a table header, and a `|` inside a link on it a cell
/// border.
fn may_become_table_header(text: &str, container: &Range<usize>, at: usize) -> bool {
    let first_line_end = text[container.start..]
        .find('\n')
        .map_or(text.len(), |nl| container.start + nl);
    at < first_line_end && text[first_line_end..].matches('\n').count() < 2
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The prefix of `text` that is settled.
    fn settled(text: &str) -> &str {
        &text[..Markdown::stream_spans(text).settled]
    }

    #[test]
    fn plain_text_and_closed_links_are_settled() {
        for text in ["Hello", "See [[a]] and more", "`[[a]]` x", "[[a]] [[]] x"] {
            assert_eq!(settled(text), text);
        }
    }

    #[test]
    fn an_open_link_waits_for_its_brackets() {
        assert_eq!(settled("See [[Travel"), "See ");
        assert_eq!(settled("See [[a]"), "See ");
        assert_eq!(settled("See ["), "See ");
        // A wiki-link may span lines.
        assert_eq!(settled("See [[a\nb"), "See ");
        // `]]` after it decides it: `[[a<b]]` is no link either way.
        assert_eq!(settled("x [[]] y"), "x [[]] y");
        // A link at the very end waits for one more character.
        assert_eq!(settled("See [[a]]"), "See ");
        // A paragraph that ended decides its brackets.
        assert_eq!(settled("[[a\n\nb"), "[[a\n\nb");
    }

    #[test]
    fn a_link_after_an_open_backtick_waits_for_the_paragraph() {
        let text = "A lone ` then [[x]] and on";
        assert_eq!(settled(text), "A lone ` then ");
        let text = "A lone ` then [[x]].\n\nNext [[y]] on";
        assert_eq!(settled(text), text, "the paragraph ended");
        // In a list item the item is the container.
        let text = "- a `x\n- b [[y]] on";
        assert_eq!(settled(text), text);
        // An open `<` may become an HTML tag around the link.
        assert_eq!(settled("a <span t=\"[[x]] on"), "a <span t=\"");
    }

    #[test]
    fn a_code_span_at_the_end_waits_for_its_closing_run() {
        // "`[[a]]``" is no code span, so the link may still appear.
        assert_eq!(settled("x `[[a]]`"), "x ");
        assert_eq!(settled("x `[[a]]` y"), "x `[[a]]` y");
    }

    #[test]
    fn a_potholed_link_on_a_first_line_waits_for_the_second() {
        assert_eq!(settled("[[a|b]] c"), "");
        assert_eq!(settled("[[a|b]] c\n|-|"), "");
        assert_eq!(settled("[[a|b]] c\nmore\n"), "[[a|b]] c\nmore\n");
        assert_eq!(settled("x\n[[a|b]] c"), "x\n[[a|b]] c");
        assert_eq!(settled("[[a]] c"), "[[a]] c");
    }

    #[test]
    fn code_blocks_are_settled() {
        let text = "```\n[[a";
        assert_eq!(settled(text), text);
        let text = "- item\n\n      [[x]]";
        assert_eq!(settled(text), text);
        assert!(Markdown::stream_spans(text).spans.links.is_empty());
    }

    /// The ranges of the links of `spans` that end within `len`.
    fn links_within(spans: &LinkSpans, len: usize) -> Vec<Range<usize>> {
        spans
            .links
            .iter()
            .filter(|link| link.range.end <= len)
            .map(|link| link.range.clone())
            .collect()
    }

    #[test]
    fn what_is_settled_never_changes_as_the_text_grows() {
        let texts = [
            "See [[Travel log]] and [[Packing list|the list]].",
            "Run `[[code]]` and ``a ` [[b]]`` then [[c]].",
            "A lone ` then [[x]].\n\nNext [[y]].",
            "Before [[a]]\n```rust\nlet x = \"[[b]]\";\n```\nAfter [[c]]\n",
            "~~~~\n[[a]]\n~~~\n[[b]]\n~~~~\n[[c]]",
            "> ```\n> [[a]]\n> ```\n> [[b]]",
            "- item\n\n      [[x]]\n\n- [[y]]",
            "- item\n\n    more [[a]]\n    - nested [[b]]",
            "Para\n    [[a]]\n\n    [[code]]\n",
            "[[a|b]] c\n|-|-|\n| [[x|y]] | z |\n",
            "a <span t=\"[[x]]\"> [[y]] <b>",
            "`[[a]]`` [[b]] `` c\n\n![[embed]] \\[[x]] [[a\nb]]",
        ];
        for text in texts {
            let full = Markdown::link_spans(text);
            let mut boundaries: Vec<usize> = (0..=text.len())
                .filter(|&ix| text.is_char_boundary(ix))
                .collect();
            boundaries.dedup();
            for at in boundaries {
                let prefix = &text[..at];
                let stream = Markdown::stream_spans(prefix);
                let settled = stream.settled;
                assert!(
                    stream
                        .spans
                        .link_at(settled)
                        .is_none_or(|l| l.range.start == settled),
                    "settled inside a link in {prefix:?}"
                );
                assert!(
                    !full
                        .links
                        .iter()
                        .any(|l| l.range.start < settled && l.range.end > settled),
                    "a link of {text:?} starts before what {prefix:?} settles"
                );
                assert_eq!(
                    links_within(&stream.spans, settled),
                    links_within(&full, settled),
                    "the links of {:?} change in {text:?}",
                    &prefix[..settled]
                );
            }
        }
    }

    /// Whether the `x` of `text` + `closing` + `x` is code.
    fn x_after_is_code(text: &str, closing: &str) -> bool {
        let probe = format!("{text}{closing}x");
        Markdown::link_spans(&probe).in_code(probe.len() - 1)
    }

    #[test]
    fn an_open_fence_is_closed_like_it_was_opened() {
        for (text, closing) in [
            ("```rust\ncode", "\n```\n"),
            ("~~~~\ncode\n", "~~~~\n"),
            ("  ```\ncode\n```x", "\n  ```\n"),
            ("Text\n\n````\n```\n", "````\n"),
        ] {
            assert_eq!(
                Markdown::closing_fence(text).as_deref(),
                Some(closing),
                "{text:?}"
            );
            assert!(!x_after_is_code(text, closing), "{text:?}");
        }
        for text in [
            "```\ncode\n```",
            "```\ncode\n```\n",
            "Text",
            "    code",
            "> ```\n> q",
        ] {
            assert_eq!(Markdown::closing_fence(text), None, "{text:?}");
            assert!(!x_after_is_code(text, "\n"), "{text:?}");
        }
    }
}

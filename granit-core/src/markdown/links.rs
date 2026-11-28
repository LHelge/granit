//! Where a body's wiki-links, code and heading anchors are, as byte ranges,
//! for an editor that marks, follows and completes links in the raw text.
//!
//! One pulldown-cmark pass with the renderer's options, so the editor, the
//! backlink index ([`Markdown::wiki_link_targets`]), the anchor index
//! ([`Markdown::anchor_ids`]) and link renaming see the same links.

use std::ops::Range;

use pulldown_cmark::{Event, LinkType, Options, Parser, Tag};

use super::render::base_options;
use super::Markdown;

/// The wiki-links, code and heading anchors of one text, in document order.
/// Every range is a byte range into the text passed to
/// [`Markdown::link_spans`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LinkSpans {
    /// `[[target]]` and `[[target|label]]` outside code and raw HTML.
    pub links: Vec<WikiLinkSpan>,
    /// Where wiki-link syntax is literal text: code blocks (fences
    /// included), inline code spans (backticks included) and raw HTML
    /// blocks. Sorted and non-overlapping.
    pub code: Vec<Range<usize>>,
    /// Headings that declare a `{#id}` attribute, ATX and setext.
    pub anchors: Vec<AnchorHeading>,
}

/// One wiki-link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WikiLinkSpan {
    /// The whole link, brackets included.
    pub range: Range<usize>,
    /// The target as the renderer resolves it: the part before an optional
    /// `|label`.
    pub target: String,
    /// The text after the `|`, if there is one.
    pub label: Option<String>,
}

/// A heading that declares `{#id}`, a wiki-link target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnchorHeading {
    pub id: String,
    /// The whole heading; a setext heading's starts at its text line.
    pub range: Range<usize>,
}

impl LinkSpans {
    /// The wiki-link covering byte `offset`, if any.
    pub fn link_at(&self, offset: usize) -> Option<&WikiLinkSpan> {
        let ix = self.links.partition_point(|link| link.range.end <= offset);
        self.links
            .get(ix)
            .filter(|link| link.range.contains(&offset))
    }

    /// Whether byte `offset` lies in code or raw HTML (see [`Self::code`]).
    pub fn in_code(&self, offset: usize) -> bool {
        let ix = self.code.partition_point(|range| range.end <= offset);
        self.code
            .get(ix)
            .is_some_and(|range| range.contains(&offset))
    }

    /// The start of the heading that declares `id`, compared
    /// case-insensitively like [`crate::cave::Cave::resolve_link`].
    pub fn anchor_offset(&self, id: &str) -> Option<usize> {
        let want = id.to_lowercase();
        self.anchors
            .iter()
            .find(|anchor| anchor.id.to_lowercase() == want)
            .map(|anchor| anchor.range.start)
    }
}

/// The options every wiki-link aware pass parses with: the renderer's.
pub(super) fn link_options() -> Options {
    let mut options = base_options();
    options.insert(Options::ENABLE_WIKILINKS);
    options
}

impl Markdown<'_> {
    /// Find the wiki-links, code and heading anchors of `text` (a note body,
    /// or a whole file: frontmatter holds none of them). Offsets are into
    /// `text`.
    pub fn link_spans(text: &str) -> LinkSpans {
        let mut spans = LinkSpans::default();
        // A code block or HTML block covers everything up to its end, so
        // events inside it are skipped.
        let mut covered_until = 0;
        for (event, range) in Parser::new_ext(text, link_options()).into_offset_iter() {
            if range.start < covered_until {
                continue;
            }
            match event {
                Event::Start(Tag::Link {
                    link_type: LinkType::WikiLink { has_pothole },
                    dest_url,
                    ..
                }) => {
                    let label = has_pothole
                        .then(|| text[range.clone()].strip_suffix("]]"))
                        .flatten()
                        .and_then(|inner| inner.split_once('|'))
                        .map(|(_, label)| label.to_string());
                    spans.links.push(WikiLinkSpan {
                        range,
                        target: dest_url.to_string(),
                        label,
                    });
                }
                Event::Start(Tag::Heading { id: Some(id), .. }) => {
                    spans.anchors.push(AnchorHeading {
                        id: id.to_string(),
                        range,
                    });
                }
                Event::Start(Tag::CodeBlock(_) | Tag::HtmlBlock) => {
                    covered_until = range.end;
                    spans.code.push(range);
                }
                Event::Code(_) => spans.code.push(range),
                _ => {}
            }
        }
        spans
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn targets(text: &str) -> Vec<String> {
        Markdown::link_spans(text)
            .links
            .into_iter()
            .map(|link| link.target)
            .collect()
    }

    fn code(text: &str) -> Vec<&str> {
        Markdown::link_spans(text)
            .code
            .into_iter()
            .map(|range| &text[range])
            .collect()
    }

    #[test]
    fn links_carry_target_label_and_byte_range() {
        let text = "Åäö [[Räksmörgås och kaffe|fika]] and [[Test note]].";
        let spans = Markdown::link_spans(text);
        assert_eq!(spans.links.len(), 2);
        let fika = &spans.links[0];
        assert_eq!(&text[fika.range.clone()], "[[Räksmörgås och kaffe|fika]]");
        assert_eq!(fika.target, "Räksmörgås och kaffe");
        assert_eq!(fika.label.as_deref(), Some("fika"));
        assert_eq!(spans.links[1].label, None);
        assert_eq!(
            spans.link_at(fika.range.start).unwrap().target,
            "Räksmörgås och kaffe"
        );
        assert_eq!(spans.link_at(fika.range.end), None);
        assert_eq!(spans.link_at(0), None);
        assert_eq!(
            spans.link_at(text.len() - 3).unwrap().target,
            "Test note",
            "inside the second link"
        );
    }

    #[test]
    fn links_are_what_the_renderer_links() {
        // Empty targets are text, embeds are images, and the target is not
        // trimmed (the resolver sees it as written).
        assert_eq!(
            targets("[[ spaced | label ]] [[]] [[|x]] ![[embed]] [[[a]]"),
            [" spaced ", "a"]
        );
        let md = "[[a]] `[[b]]` [[c|d]]\n\n```\n[[e]]\n```\n";
        assert_eq!(targets(md), Markdown::new(md).wiki_link_targets());
    }

    #[test]
    fn code_covers_fences_indented_blocks_and_spans() {
        let text = "a `b` c\n```rust\nx\n```\nd ``e ` f`` g `h\n~~~~\ny\n~~~\nz\n";
        assert_eq!(
            code(text),
            ["`b`", "```rust\nx\n```", "``e ` f``", "~~~~\ny\n~~~\nz\n"]
        );
        let text = "Para\n\n    [[a]] code\n\tmore\n\nAfter [[b]]\n";
        // An indented block starts after the first line's indentation.
        assert_eq!(code(text), ["[[a]] code\n\tmore\n"]);
        assert_eq!(targets(text), ["b"]);
        // Indentation that continues a paragraph or a list item is not code.
        assert!(code("Para\n    [[a]]\n").is_empty());
        assert!(code("- item\n\n    [[a]]\n").is_empty());
        // A code span may wrap within its paragraph, not across blocks.
        let text = "One `[[a]]\nstill code` and [[b]].\n\nNew `para\n\nnot` [[c]]\n";
        assert_eq!(code(text), ["`[[a]]\nstill code`"]);
        assert_eq!(targets(text), ["b", "c"]);
    }

    #[test]
    fn html_blocks_hold_no_links() {
        let text = "<div>\n[[x]]\n</div>\n\n[[y]]";
        assert_eq!(targets(text), ["y"]);
        let spans = Markdown::link_spans(text);
        assert!(spans.in_code(text.find("[[x").unwrap()));
        assert!(!spans.in_code(text.find("[[y").unwrap()));
    }

    #[test]
    fn blockquotes_hold_links_and_indented_code() {
        let text = "> See [[a]]\n>\n>     [[b]]\n";
        assert_eq!(targets(text), ["a"]);
        let spans = Markdown::link_spans(text);
        assert!(spans.in_code(text.find("[[b").unwrap()));
        assert!(!spans.in_code(text.find("[[a").unwrap()));
    }

    #[test]
    fn anchors_are_atx_and_setext_headings_with_an_id() {
        let text = "Intro\n\n## Volvo {#volvo}\n\nSaab {#Saab-anchor .cls}\n----\n\n\
                    # Plain\n\n```\n# Code {#code}\n```\n\n> # Quoted {#quoted}\n";
        let spans = Markdown::link_spans(text);
        let ids: Vec<&str> = spans.anchors.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["volvo", "Saab-anchor", "quoted"]);
        assert_eq!(ids, Markdown::new(text).anchor_ids());
        let at = spans.anchor_offset("VOLVO").unwrap();
        assert!(text[at..].starts_with("## Volvo"));
        let at = spans.anchor_offset("saab-anchor").unwrap();
        assert!(
            text[at..].starts_with("Saab {#"),
            "setext starts at its text"
        );
        let at = spans.anchor_offset("quoted").unwrap();
        assert!(text[at..].starts_with("# Quoted"));
        assert_eq!(spans.anchor_offset("code"), None);
        assert_eq!(spans.anchor_offset("plain"), None);
        assert_eq!(Markdown::link_spans("#A {#a}\n").anchor_offset("a"), None);
        assert_eq!(
            Markdown::link_spans("x\n\n# Ö {#ö}\n").anchor_offset("Ö"),
            Some(3)
        );
    }

    #[test]
    fn in_code_handles_unclosed_constructs() {
        // An unclosed fence runs to the end; an unmatched backtick is text.
        let text = "```\n[[a";
        assert!(Markdown::link_spans(text).in_code(4));
        assert!(!Markdown::link_spans("`[[a").in_code(1));
        assert!(Markdown::link_spans("`[[a` b`").in_code(1));
    }

    /// A body the size of the largest note in daily use (6,907 lines,
    /// ~226 KB): mixed paragraphs, links, headings, lists and code.
    fn large_body() -> String {
        let mut body = String::new();
        let mut lines = 0;
        let mut n = 0;
        while lines < 6907 {
            n += 1;
            let section = format!(
                "## Section {n} {{#section-{n}}}\n\n\
                 Paragraph {n} links [[Note {n}]] and [[Other|label {n}]] with `code {n}`\n\
                 and continues with some **bold** text, *emphasis* and a [link](https://x.se),\n\
                 then rambles on for a while so that the line is about as long as prose is.\n\n\
                 - item [[List {n}]] with a little more text after the link itself\n\
                 - [ ] task {n} that also has a description long enough to wrap once\n\n\
                 ```rust\nfn f{n}() {{ \"[[not a link]]\" }}\n```\n\n\
                 > Quote with [[Quoted {n}]], attributed to nobody in particular.\n\n"
            );
            lines += section.matches('\n').count();
            body.push_str(&section);
        }
        body
    }

    #[test]
    fn a_large_body_is_scanned_per_edit_fast() {
        let body = large_body();
        let start = std::time::Instant::now();
        let spans = Markdown::link_spans(&body);
        let elapsed = start.elapsed();
        eprintln!(
            "link_spans: {} lines, {} bytes in {elapsed:?}",
            body.lines().count(),
            body.len()
        );
        let sections = spans.anchors.len();
        assert_eq!(spans.links.len(), sections * 4);
        assert_eq!(spans.code.len(), sections * 2);
        assert!(spans
            .anchor_offset(&format!("section-{sections}"))
            .is_some());
    }
}

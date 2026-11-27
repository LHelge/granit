//! Blockquote alert completion, like the web editor's: typing `> [!` at the
//! start of a line offers the GitHub alert types (`[!NOTE]`, `[!TIP]`, …),
//! and accepting one inserts it with a `> ` continuation line for the alert's
//! text. Kept free of GPUI; the `[[` popup in [`super::links`] shows the
//! entries.

use std::ops::Range;

use granit_core::markdown::LinkSpans;

use super::wiki_links::{Candidate, CandidateKind};

/// The alert types, with what each is for.
pub const ALERT_TYPES: [(&str, &str); 5] = [
    ("NOTE", "Useful information that users should know"),
    (
        "TIP",
        "Helpful advice for doing things better or more easily",
    ),
    (
        "IMPORTANT",
        "Key information users need to know to achieve their goal",
    ),
    ("WARNING", "Urgent info that needs immediate user attention"),
    (
        "CAUTION",
        "Advises about risks or negative outcomes of certain actions",
    ),
];

/// An alert marker being typed: `> [!` and a partial type before the caret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlertQuery {
    /// Byte offset of the `[!`.
    pub start: usize,
    /// The type typed so far, between the `[!` and the caret.
    pub typed: String,
}

/// The alert marker being typed at `cursor`: the caret's line up to the
/// caret is `>`, optional whitespace, `[!` and word characters (the web
/// editor's `^>\s*\[!(\w*)$`), outside code (`spans` are those of `text`).
pub fn alert_query(text: &str, spans: &LinkSpans, cursor: usize) -> Option<AlertQuery> {
    let cursor = cursor.min(text.len());
    if !text.is_char_boundary(cursor) {
        return None;
    }
    let line_start = text[..cursor].rfind('\n').map_or(0, |at| at + 1);
    let before = text[line_start..cursor].strip_prefix('>')?;
    let typed = before.trim_start().strip_prefix("[!")?;
    if !typed.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    let start = cursor - typed.len() - 2;
    if spans.in_code(start) {
        return None;
    }
    Some(AlertQuery {
        start,
        typed: typed.to_string(),
    })
}

/// The alert types whose name starts with `typed`, ignoring case, as popup
/// entries labelled `[!TYPE]`.
pub fn complete(typed: &str) -> Vec<Candidate> {
    let typed = typed.to_uppercase();
    ALERT_TYPES
        .iter()
        .filter(|(name, _)| name.starts_with(&typed))
        .map(|&(name, detail)| Candidate {
            label: format!("[!{name}]"),
            kind: CandidateKind::Alert { detail },
        })
        .collect()
}

/// The edit that accepts `label` (`[!NOTE]`) for the marker at `start`:
/// replace the typed `[!…` up to the caret with the label and a new `> `
/// line. The caret belongs at the end of the replacement.
pub fn completion_edit(start: usize, cursor: usize, label: &str) -> (Range<usize>, String) {
    (start..cursor.max(start), format!("{label}\n> "))
}

#[cfg(test)]
mod tests {
    use granit_core::markdown::Markdown;

    use super::*;

    fn query(text: &str) -> Option<AlertQuery> {
        alert_query(text, &Markdown::link_spans(text), text.len())
    }

    #[test]
    fn a_marker_after_a_quote_is_completed() {
        assert_eq!(
            query("text\n> [!"),
            Some(AlertQuery {
                start: 7,
                typed: String::new()
            })
        );
        assert_eq!(query(">[!wa").unwrap().typed, "wa");
        assert_eq!(query(">  [!NO").unwrap().start, 3);
        for text in ["[!", "a > [!", "> x [!", "> [!no te", "  > [!", "> [[!"] {
            assert_eq!(query(text), None, "{text:?}");
        }
    }

    #[test]
    fn no_marker_completion_inside_code() {
        assert_eq!(query("```\n> [!"), None);
    }

    #[test]
    fn types_filter_by_prefix_ignoring_case() {
        let labels = |typed| {
            complete(typed)
                .into_iter()
                .map(|c| c.label)
                .collect::<Vec<_>>()
        };
        assert_eq!(labels("").len(), 5);
        assert_eq!(labels("wa"), ["[!WARNING]"]);
        assert_eq!(labels("I"), ["[!IMPORTANT]"]);
        assert!(labels("x").is_empty());
    }

    #[test]
    fn accepting_inserts_the_marker_and_a_quote_line() {
        let text = "> [!ti";
        let (range, insert) = completion_edit(2, text.len(), "[!TIP]");
        let mut out = text.to_string();
        out.replace_range(range, &insert);
        assert_eq!(out, "> [!TIP]\n> ");
    }
}

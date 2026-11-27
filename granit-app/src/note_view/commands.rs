//! The web editor's own shortcuts on top of zorite-editor's formatting
//! keys: insert a link (`Mod-k`) and toggle a task checkbox (`Mod-l`). Bold,
//! italic and the rest come with the editor.

use std::ops::Range;

use gpui::{Context, EntityInputHandler, Window};

use super::wiki_links::byte_offset_from_utf16;
use super::{InsertLink, NoteView, ToggleTask};

/// One edit of the document: replace `range` with `text`, then put the caret
/// at `caret`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    pub range: Range<usize>,
    pub text: String,
    pub caret: usize,
}

/// Whether `text` looks like a URL to link to rather than a note name.
fn is_url(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    ["http://", "https://", "mailto:"]
        .iter()
        .any(|scheme| lower.starts_with(scheme))
        && !text.contains(char::is_whitespace)
}

/// Wrap the selection as a link, like the web editor: plain text becomes a
/// wiki-link, a selected URL a markdown link with the caret in its empty
/// label, and an empty selection inserts `[[]]` with the caret inside.
/// Returns the edit and whether to open link completion.
pub fn insert_link(text: &str, selection: Range<usize>) -> (Edit, bool) {
    let selected = &text[selection.clone()];
    if selected.is_empty() {
        let edit = Edit {
            caret: selection.start + 2,
            range: selection,
            text: "[[]]".into(),
        };
        return (edit, true);
    }
    let edit = if is_url(selected) {
        Edit {
            text: format!("[]({selected})"),
            caret: selection.start + 1,
            range: selection,
        }
    } else {
        Edit {
            text: format!("[[{selected}]]"),
            caret: selection.end + 4,
            range: selection,
        }
    };
    (edit, false)
}

/// The length of a list marker with its trailing whitespace at the start
/// of `rest` (`- `, `* `, `+ `, `1. `, `1) `), if there is one.
fn list_marker_len(rest: &str) -> Option<usize> {
    let marker = if rest.starts_with(['-', '*', '+']) {
        1
    } else {
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 || !rest[digits..].starts_with(['.', ')']) {
            return None;
        }
        digits + 1
    };
    let spaces = rest[marker..]
        .bytes()
        .take_while(|b| *b == b' ' || *b == b'\t')
        .count();
    (spaces > 0).then_some(marker + spaces)
}

/// Toggle the task checkbox on every line the selection touches, like the
/// web editor: `[ ]` and `[x]` flip, a list item gets `[ ] `, any other line
/// becomes `- [ ] `. The caret stays on its text.
pub fn toggle_tasks(text: &str, selection: Range<usize>, caret: usize) -> Edit {
    let start = text[..selection.start].rfind('\n').map_or(0, |at| at + 1);
    let end = text[selection.end..]
        .find('\n')
        .map_or(text.len(), |at| selection.end + at);
    let mut out = String::with_capacity(end - start + 8);
    let mut new_caret = caret;
    let mut line_start = start;
    for (ix, line) in text[start..end].split('\n').enumerate() {
        if ix > 0 {
            out.push('\n');
        }
        let indent = line.len() - line.trim_start_matches([' ', '\t']).len();
        let rest = &line[indent..];
        // (offset in the line, bytes removed, text inserted)
        let (at, removed, insert) = match list_marker_len(rest) {
            Some(marker) => {
                let after = &rest[marker..];
                let state = after.as_bytes().get(1).copied();
                let is_box = after.starts_with('[')
                    && matches!(state, Some(b' ' | b'x' | b'X'))
                    && after[2..].starts_with(']')
                    && after[3..].chars().next().is_none_or(char::is_whitespace);
                if is_box {
                    let flipped = if state == Some(b' ') { "x" } else { " " };
                    (indent + marker + 1, 1, flipped)
                } else {
                    (indent + marker, 0, "[ ] ")
                }
            }
            None => (indent, 0, "- [ ] "),
        };
        out.push_str(&line[..at]);
        out.push_str(insert);
        out.push_str(&line[at + removed..]);
        let edit_at = line_start + at;
        if caret > edit_at || (caret == edit_at && removed == 0) {
            new_caret = (new_caret + insert.len()).saturating_sub(removed);
        }
        line_start += line.len() + 1;
    }
    Edit {
        range: start..end,
        text: out,
        caret: new_caret,
    }
}

impl NoteView {
    /// The editor's selection as a byte range, and its caret.
    fn selection(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(Range<usize>, usize)> {
        let editor = self.editor.clone()?;
        editor.update(cx, |editor, cx| {
            let selection = editor.selected_text_range(false, window, cx)?;
            let text = editor.text();
            let range = byte_offset_from_utf16(text, selection.range.start)
                ..byte_offset_from_utf16(text, selection.range.end);
            Some((range, editor.cursor()))
        })
    }

    /// Apply `edit` as one undoable editor step and record it as a user
    /// edit.
    fn apply_edit(&mut self, edit: Edit, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = self.editor.clone() else {
            return;
        };
        editor.update(cx, |editor, cx| {
            editor.replace_range(edit.range, &edit.text, cx);
            editor.set_cursor(edit.caret, cx);
        });
        // `replace_range` does not emit `Changed`.
        self.note_edited(window, cx);
        self.mark_broken_links(cx);
    }

    pub(super) fn on_insert_link(
        &mut self,
        _: &InsertLink,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((selection, _)) = self.selection(window, cx) else {
            return;
        };
        let Some(text) = self.text(cx) else {
            return;
        };
        let (edit, complete) = insert_link(text, selection);
        self.apply_edit(edit, window, cx);
        if complete {
            self.update_completion(true, window, cx);
        }
    }

    pub(super) fn on_toggle_task(
        &mut self,
        _: &ToggleTask,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((selection, caret)) = self.selection(window, cx) else {
            return;
        };
        let Some(text) = self.text(cx) else {
            return;
        };
        let edit = toggle_tasks(text, selection, caret);
        self.apply_edit(edit, window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(text: &str, edit: &Edit) -> String {
        let mut out = text.to_string();
        out.replace_range(edit.range.clone(), &edit.text);
        out
    }

    #[test]
    fn insert_link_wraps_text_urls_and_nothing() {
        let (edit, complete) = insert_link("see Volvo now", 4..9);
        assert_eq!(apply("see Volvo now", &edit), "see [[Volvo]] now");
        assert_eq!(edit.caret, 13);
        assert!(!complete);

        let text = "go https://x.se now";
        let (edit, _) = insert_link(text, 3..15);
        assert_eq!(apply(text, &edit), "go [](https://x.se) now");
        assert_eq!(edit.caret, 4);

        let (edit, complete) = insert_link("ab", 1..1);
        assert_eq!(apply("ab", &edit), "a[[]]b");
        assert_eq!(edit.caret, 3);
        assert!(complete);
    }

    #[test]
    fn toggle_tasks_flips_adds_and_converts() {
        let cases = [
            ("- [ ] open", "- [x] open"),
            ("- [x] done", "- [ ] done"),
            ("  * [X] Done", "  * [ ] Done"),
            ("- item", "- [ ] item"),
            ("1. first", "1. [ ] first"),
            ("plain", "- [ ] plain"),
            ("  indented", "  - [ ] indented"),
            ("", "- [ ] "),
            ("- [ ]", "- [x]"),
        ];
        for (line, want) in cases {
            let edit = toggle_tasks(line, 0..0, 0);
            assert_eq!(apply(line, &edit), want, "{line:?}");
        }
    }

    #[test]
    fn toggle_tasks_covers_the_selected_lines_and_keeps_the_caret() {
        let text = "a\n- b\n- [x] c\nd";
        let edit = toggle_tasks(text, 3..10, 10);
        assert_eq!(apply(text, &edit), "a\n- [ ] b\n- [ ] c\nd");
        assert_eq!(edit.range, 2..13);

        // The caret after the inserted prefix moves with its text.
        let text = "word";
        let edit = toggle_tasks(text, 2..2, 2);
        assert_eq!(edit.caret, 8);
        assert_eq!(&apply(text, &edit)[edit.caret..], "rd");
        // A flipped box keeps the caret.
        let text = "- [ ] x";
        assert_eq!(toggle_tasks(text, 7..7, 7).caret, 7);
    }
}

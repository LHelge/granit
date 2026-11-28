//! Find and replace in note: a bar above the editor, the find field with
//! its match count and step buttons over the replace field with "Replace"
//! and "All". Matches are case-insensitive (`zorite_editor::find_in_source`)
//! and highlighted by the editor (`set_search`); Enter and Shift-Enter (or
//! the find-next and find-previous keys) step through them, Enter in the
//! replace field replaces the current one, Escape closes the bar. The bar
//! opens with the selected text, else the last query.

use std::ops::Range;

use gpui::{
    div, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, ParentElement,
    SharedString, Styled, Subscription, Window,
};
use gpui_kit::component::{
    button::{Button, ButtonVariants},
    h_flex,
    input::{self, Input, InputEvent, InputState},
    v_flex, ActiveTheme, Disableable as _, IconName, Sizable,
};
use zorite_editor::EditorState;

use super::commands::Edit;
use super::{FindInNote, FindNext, FindPrevious, NoteView, REVEAL_ATTEMPTS};

/// The open find bar.
pub struct FindBar {
    input: Entity<InputState>,
    replace: Entity<InputState>,
    /// The query the matches are for.
    query: SharedString,
    matches: Vec<Range<usize>>,
    /// Index into `matches` of the current match.
    active: Option<usize>,
    _events: Vec<Subscription>,
}

/// The edit that replaces every match in `text` with `replacement`: one
/// range from the first match to the end of the last, so it is a single
/// undo step, with the caret after the last replacement. `None` without
/// matches.
pub fn replace_all_edit(text: &str, matches: &[Range<usize>], replacement: &str) -> Option<Edit> {
    let (first, last) = (matches.first()?, matches.last()?);
    let mut out = String::new();
    let mut at = first.start;
    for range in matches {
        out.push_str(&text[at..range.start]);
        out.push_str(replacement);
        at = range.end;
    }
    let caret = first.start + out.len();
    Some(Edit {
        range: first.start..last.end,
        text: out,
        caret,
    })
}

impl FindBar {
    /// Search the editor's text again for the typed query (after an edit or
    /// a note switch), keeping the current match where it can.
    pub fn refresh(&mut self, editor: Option<&Entity<EditorState>>, cx: &mut gpui::App) {
        let query = self.input.read(cx).value();
        let Some(editor) = editor else {
            self.clear_matches();
            return;
        };
        self.matches = zorite_editor::find_in_source(editor.read(cx).text(), &query);
        self.active = match self.active {
            _ if self.matches.is_empty() => None,
            Some(active) => Some(active.min(self.matches.len() - 1)),
            None => Some(0),
        };
        let (matches, active) = (self.matches.clone(), self.active);
        editor.update(cx, |editor, cx| editor.set_search(matches, active, cx));
    }

    pub fn clear_matches(&mut self) {
        self.matches.clear();
        self.active = None;
    }

    /// "2 of 5", "No matches", or nothing without a query.
    fn summary(&self, query: &str) -> SharedString {
        match self.active {
            _ if query.is_empty() => SharedString::default(),
            Some(active) => format!("{} of {}", active + 1, self.matches.len()).into(),
            None => "No matches".into(),
        }
    }
}

impl NoteView {
    /// The find bar's matches and the current one, while it is open.
    #[cfg(test)]
    pub(super) fn find_state(&self) -> Option<(Vec<Range<usize>>, Option<usize>)> {
        let find = self.find.as_ref()?;
        Some((find.matches.clone(), find.active))
    }

    /// Type `query` into the find bar, for tests.
    #[cfg(test)]
    pub(super) fn set_find_query(
        &mut self,
        query: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(find) = &self.find {
            find.input.update(cx, |input, cx| {
                input.set_value(query.to_string(), window, cx)
            });
        }
        self.search(cx);
    }

    #[cfg(test)]
    pub(super) fn find_input_focus(&self, window: &Window, cx: &gpui::App) -> bool {
        use gpui::Focusable as _;
        self.find
            .as_ref()
            .is_some_and(|f| f.input.read(cx).focus_handle(cx).is_focused(window))
    }

    /// Type `text` into the replace field (focusing it), for tests.
    #[cfg(test)]
    pub(super) fn set_replace_text(
        &mut self,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(find) = &self.find {
            find.replace.update(cx, |input, cx| {
                input.set_value(text.to_string(), window, cx);
                input.focus(window, cx);
            });
        }
    }

    pub(super) fn on_find(&mut self, _: &FindInNote, window: &mut Window, cx: &mut Context<Self>) {
        self.open_find(window, cx);
    }

    /// Step to the next match; with the bar closed, open it (on the last
    /// query) at the first match after the caret.
    pub(super) fn on_find_next(
        &mut self,
        _: &FindNext,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.find.is_some() {
            self.step_match(1, cx);
        } else {
            self.open_find(window, cx);
        }
    }

    /// Step to the previous match; with the bar closed, open it at the last
    /// match before the caret.
    pub(super) fn on_find_previous(
        &mut self,
        _: &FindPrevious,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.find.is_none() {
            self.open_find(window, cx);
        }
        self.step_match(-1, cx);
    }

    /// Open the find bar (or focus it again) with the query selected. It is
    /// seeded with the selected text when that is one non-empty line, else
    /// (when it opens) with the last closed bar's query.
    pub fn open_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editor.is_none() {
            return;
        }
        let selected = self
            .selection(window, cx)
            .and_then(|(range, _)| Some(self.text(cx)?.get(range)?.to_string()))
            .filter(|text| !text.is_empty() && !text.contains('\n'));
        let mut seed = selected.map(SharedString::from);
        if self.find.is_none() {
            let input = cx.new(|cx| InputState::new(window, cx).placeholder("Find in note"));
            let replace = cx.new(|cx| InputState::new(window, cx).placeholder("Replace with"));
            let events = vec![
                cx.subscribe_in(
                    &input,
                    window,
                    |this, _, event: &InputEvent, _, cx| match event {
                        InputEvent::Change => this.search(cx),
                        InputEvent::PressEnter { shift, .. } => {
                            this.step_match(if *shift { -1 } else { 1 }, cx)
                        }
                        InputEvent::Focus | InputEvent::Blur => {}
                    },
                ),
                cx.subscribe_in(
                    &replace,
                    window,
                    |this, _, event: &InputEvent, window, cx| {
                        if let InputEvent::PressEnter { .. } = event {
                            this.replace_current(window, cx);
                        }
                    },
                ),
            ];
            self.find = Some(FindBar {
                input,
                replace,
                query: SharedString::default(),
                matches: Vec::new(),
                active: None,
                _events: events,
            });
            if seed.is_none() && !self.last_find_query.is_empty() {
                seed = Some(self.last_find_query.clone());
            }
        }
        if let Some(find) = &self.find {
            find.input.update(cx, |input, cx| {
                if let Some(seed) = seed {
                    input.set_value(seed, window, cx);
                }
                input.focus(window, cx);
                input.select_all(window, cx);
            });
        }
        self.search(cx);
        cx.notify();
    }

    /// Close the find bar, clear the highlights and return to the editor.
    pub fn close_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(find) = self.find.take() else {
            return;
        };
        self.last_find_query = find.input.read(cx).value();
        if let Some(editor) = &self.editor {
            editor.update(cx, |editor, cx| {
                editor.set_search(Vec::new(), None, cx);
                editor.focus(window, cx);
            });
        }
        cx.notify();
    }

    /// Search for the typed query and show the first match after the caret.
    fn search(&mut self, cx: &mut Context<Self>) {
        let (Some(find), Some(editor)) = (&mut self.find, &self.editor) else {
            return;
        };
        let query = find.input.read(cx).value();
        if query == find.query {
            // A change event without a new query (the input reports some
            // late): keep the current match.
            return;
        }
        find.query = query;
        let cursor = editor.read(cx).cursor();
        find.active = None;
        find.refresh(Some(editor), cx);
        if let Some(first) = find.matches.iter().position(|m| m.start >= cursor) {
            find.active = Some(first);
            let (matches, active) = (find.matches.clone(), find.active);
            editor.update(cx, |editor, cx| editor.set_search(matches, active, cx));
        }
        self.scroll_to_active_match(cx);
    }

    /// Make the next (`delta` 1) or previous (-1) match current, wrapping.
    pub(super) fn step_match(&mut self, delta: isize, cx: &mut Context<Self>) {
        let (Some(find), Some(editor)) = (&mut self.find, &self.editor) else {
            return;
        };
        if find.matches.is_empty() {
            return;
        }
        let len = find.matches.len() as isize;
        let next = find
            .active
            .map_or(0, |active| (active as isize + delta).rem_euclid(len));
        find.active = Some(next as usize);
        let (matches, active) = (find.matches.clone(), find.active);
        editor.update(cx, |editor, cx| editor.set_search(matches, active, cx));
        self.scroll_to_active_match(cx);
    }

    /// Replace the current match with the replace field's text (one undo
    /// step) and make the next match after it current.
    pub(super) fn replace_current(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(find) = &self.find else {
            return;
        };
        let Some(range) = find.active.and_then(|ix| find.matches.get(ix)).cloned() else {
            return;
        };
        let text = find.replace.read(cx).value().to_string();
        let caret = range.start + text.len();
        self.apply_edit(Edit { range, text, caret }, window, cx);
        self.activate_match_from(caret, cx);
    }

    /// Replace every match with the replace field's text, as one undo step.
    pub(super) fn replace_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(find), Some(text)) = (&self.find, self.text(cx)) else {
            return;
        };
        let replacement = find.replace.read(cx).value();
        let Some(edit) = replace_all_edit(text, &find.matches, &replacement) else {
            return;
        };
        let caret = edit.caret;
        self.apply_edit(edit, window, cx);
        self.activate_match_from(caret, cx);
    }

    /// Make the first match at or after `offset` current (wrapping to the
    /// first one) after the matches were refreshed by an edit.
    fn activate_match_from(&mut self, offset: usize, cx: &mut Context<Self>) {
        let (Some(find), Some(editor)) = (&mut self.find, &self.editor) else {
            return;
        };
        find.active = match find.matches.iter().position(|m| m.start >= offset) {
            Some(ix) => Some(ix),
            None if find.matches.is_empty() => None,
            None => Some(0),
        };
        let (matches, active) = (find.matches.clone(), find.active);
        editor.update(cx, |editor, cx| editor.set_search(matches, active, cx));
        self.scroll_to_active_match(cx);
    }

    /// Scroll the current match to the top once it is painted. The caret
    /// stays where it is; the find bar keeps focus.
    fn scroll_to_active_match(&mut self, cx: &mut Context<Self>) {
        let Some(find) = &self.find else {
            return;
        };
        if let Some(range) = find.active.and_then(|ix| find.matches.get(ix)) {
            self.pending_scroll = Some((range.start, REVEAL_ATTEMPTS));
        }
        cx.notify();
    }

    pub(super) fn render_find_bar(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let find = self.find.as_ref()?;
        let theme = cx.theme();
        let summary = find.summary(&find.input.read(cx).value());
        let has_match = find.active.is_some();
        let find_row = h_flex()
            .gap_1()
            .child(
                div()
                    .w(gpui::px(260.))
                    .child(Input::new(&find.input).small()),
            )
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(summary),
            )
            .child(
                Button::new("find-previous")
                    .ghost()
                    .xsmall()
                    .icon(IconName::ChevronUp)
                    .tooltip("Previous match (Shift+Enter)")
                    .on_click(cx.listener(|this, _, _, cx| this.step_match(-1, cx))),
            )
            .child(
                Button::new("find-next")
                    .ghost()
                    .xsmall()
                    .icon(IconName::ChevronDown)
                    .tooltip("Next match (Enter)")
                    .on_click(cx.listener(|this, _, _, cx| this.step_match(1, cx))),
            )
            .child(
                Button::new("find-close")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Close)
                    .tooltip("Close (Escape)")
                    .on_click(cx.listener(|this, _, window, cx| this.close_find(window, cx))),
            );
        let replace_row = h_flex()
            .gap_1()
            .child(
                div()
                    .w(gpui::px(260.))
                    .child(Input::new(&find.replace).small()),
            )
            .child(
                Button::new("replace-one")
                    .ghost()
                    .xsmall()
                    .label("Replace")
                    .tooltip("Replace the current match (Enter)")
                    .disabled(!has_match)
                    .on_click(cx.listener(|this, _, window, cx| this.replace_current(window, cx))),
            )
            .child(
                Button::new("replace-all")
                    .ghost()
                    .xsmall()
                    .label("All")
                    .tooltip("Replace all matches")
                    .disabled(!has_match)
                    .on_click(cx.listener(|this, _, window, cx| this.replace_all(window, cx))),
            );
        Some(
            v_flex()
                .flex_none()
                .gap_1()
                .px_6()
                .py_1()
                .border_b_1()
                .border_color(theme.border)
                // Escape in either field closes the bar before the input
                // handles it.
                .capture_action(cx.listener(|this, _: &input::Escape, window, cx| {
                    this.close_find(window, cx);
                    cx.stop_propagation();
                }))
                .child(find_row)
                .child(replace_row),
        )
    }
}

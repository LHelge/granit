//! Find in note: a one-line bar above the editor. Matches are
//! case-insensitive (`zorite_editor::find_in_source`) and highlighted by the
//! editor (`set_search`); Enter and Shift-Enter step through them, Escape
//! closes the bar.

use std::ops::Range;

use gpui::{
    div, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, ParentElement,
    SharedString, Styled, Subscription, Window,
};
use gpui_kit::component::{
    button::{Button, ButtonVariants},
    h_flex,
    input::{self, Input, InputEvent, InputState},
    ActiveTheme, IconName, Sizable,
};
use zorite_editor::EditorState;

use super::{FindInNote, NoteView, REVEAL_ATTEMPTS};

/// The open find bar.
pub struct FindBar {
    input: Entity<InputState>,
    /// The query the matches are for.
    query: SharedString,
    matches: Vec<Range<usize>>,
    /// Index into `matches` of the current match.
    active: Option<usize>,
    _events: Subscription,
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

    pub(super) fn on_find(&mut self, _: &FindInNote, window: &mut Window, cx: &mut Context<Self>) {
        self.open_find(window, cx);
    }

    /// Open the find bar (or focus it again) with the query selected.
    pub fn open_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editor.is_none() {
            return;
        }
        if self.find.is_none() {
            let input = cx.new(|cx| InputState::new(window, cx).placeholder("Find in note"));
            let events =
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
                );
            self.find = Some(FindBar {
                input,
                query: SharedString::default(),
                matches: Vec::new(),
                active: None,
                _events: events,
            });
        }
        if let Some(find) = &self.find {
            find.input.update(cx, |input, cx| {
                input.focus(window, cx);
                input.select_all(window, cx);
            });
        }
        cx.notify();
    }

    /// Close the find bar, clear the highlights and return to the editor.
    pub fn close_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.find.take().is_none() {
            return;
        }
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
        Some(
            h_flex()
                .flex_none()
                .gap_1()
                .px_6()
                .py_1()
                .border_b_1()
                .border_color(theme.border)
                // Escape in the field closes the bar before the input
                // handles it.
                .capture_action(cx.listener(|this, _: &input::Escape, window, cx| {
                    this.close_find(window, cx);
                    cx.stop_propagation();
                }))
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
                ),
        )
    }
}

//! Wiki-link behaviour of the workspace: following clicked links through
//! the cave resolver, scrolling to heading anchors, opening external URLs,
//! marking broken links, and the `[[` completion popup.
//!
//! The editor crate has no completion API, so the host drives the popup:
//! it watches `Changed` / `SelectionChanged`, finds the unclosed `[[` before
//! the caret, anchors a list at the editor's caret box, and intercepts the
//! editor's own Up/Down/Enter/Tab/Escape actions in the capture phase while
//! the list is open.

use gpui::{
    anchored, deferred, div, point, prelude::FluentBuilder, px, Anchor, AnyElement, Context, Div,
    EntityInputHandler, Focusable, InteractiveElement, IntoElement, MouseButton, ParentElement,
    Pixels, ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Window,
};
use gpui_kit::component::ActiveTheme;
use zorite_editor::Diagnostic;

use crate::wiki_links::{self, Candidate, CandidateKind};
use crate::{CaveStatus, OpenNote, Workspace};

/// Most entries the popup lists.
const COMPLETION_LIMIT: usize = 50;

/// Space kept above a heading scrolled to the top of the viewport.
const ANCHOR_MARGIN: Pixels = px(16.);

/// The open `[[` completion popup.
pub struct LinkCompletion {
    /// Byte offset of the `[[` being completed.
    start: usize,
    items: Vec<Candidate>,
    selected: usize,
    scroll: ScrollHandle,
}

impl LinkCompletion {
    #[cfg(test)]
    pub fn labels(&self) -> Vec<&str> {
        self.items.iter().map(|item| item.label.as_str()).collect()
    }

    #[cfg(test)]
    pub fn selected(&self) -> usize {
        self.selected
    }
}

/// Whether `url` may be handed to the OS opener: http(s) and mailto only,
/// with no whitespace or control characters. Link targets are note content,
/// so other schemes (`file:`, app handlers) are refused.
pub fn is_safe_external_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    let allowed = ["http://", "https://", "mailto:"]
        .iter()
        .any(|scheme| lower.starts_with(scheme) && lower.len() > scheme.len());
    allowed && !url.chars().any(|c| c.is_whitespace() || c.is_control())
}

impl Workspace {
    /// Underline the open note's wiki-links whose target does not resolve.
    /// The editor has one link color, so broken links get its diagnostic
    /// squiggle instead.
    pub(crate) fn mark_broken_links(&mut self, cx: &mut Context<Self>) {
        let CaveStatus::Open { links, .. } = &self.status else {
            return;
        };
        let diagnostics = if matches!(self.note, OpenNote::Loaded(_)) {
            let text = self.editor.read(cx).text();
            wiki_links::broken_link_ranges(&wiki_links::wiki_links(text), |target| {
                links.is_resolved(target)
            })
            .into_iter()
            .map(|range| Diagnostic { range })
            .collect()
        } else {
            Vec::new()
        };
        self.editor
            .update(cx, |editor, cx| editor.set_diagnostics(diagnostics, cx));
    }

    /// Open, refilter or close the popup for the caret's position. Only a
    /// text change (`typed`) opens it; a caret move keeps it in step or
    /// closes it.
    pub(crate) fn update_completion(
        &mut self,
        typed: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let CaveStatus::Open { links, .. } = &self.status else {
            return;
        };
        let query = if matches!(self.note, OpenNote::Loaded(_)) {
            let editor = self.editor.read(cx);
            wiki_links::link_query(editor.text(), editor.cursor())
        } else {
            None
        };
        let Some(query) = query else {
            self.dismissed_link = None;
            self.close_completion(cx);
            return;
        };
        if self.dismissed_link == Some(query.start) {
            return;
        }
        let reopen = self
            .completion
            .as_ref()
            .is_some_and(|completion| completion.start == query.start);
        if !typed && !reopen {
            self.close_completion(cx);
            return;
        }
        let items = links.complete(&query.query, COMPLETION_LIMIT);
        if items.is_empty() {
            self.close_completion(cx);
            return;
        }
        self.completion = Some(LinkCompletion {
            start: query.start,
            items,
            selected: 0,
            scroll: ScrollHandle::new(),
        });
        // The caret box comes from the editor's last paint; render again
        // once it reflects this edit.
        self.schedule_after_paint(window, cx);
        cx.notify();
    }

    fn close_completion(&mut self, cx: &mut Context<Self>) {
        if self.completion.take().is_some() {
            cx.notify();
        }
    }

    /// Move the popup selection by `delta`, wrapping around.
    fn move_completion(&mut self, delta: isize, cx: &mut Context<Self>) {
        let Some(completion) = &mut self.completion else {
            return;
        };
        let len = completion.items.len() as isize;
        completion.selected = (completion.selected as isize + delta).rem_euclid(len) as usize;
        completion.scroll.scroll_to_item(completion.selected);
        cx.notify();
    }

    /// Close the popup and keep it closed for this `[[`.
    fn dismiss_completion(&mut self, cx: &mut Context<Self>) {
        if let Some(completion) = self.completion.take() {
            self.dismissed_link = Some(completion.start);
            cx.notify();
        }
    }

    /// Insert the entry at `ix` (the selected one for `None`) and close the
    /// link with `]]`.
    pub(crate) fn accept_completion(
        &mut self,
        ix: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(completion) = self.completion.take() else {
            return;
        };
        let Some(item) = completion.items.get(ix.unwrap_or(completion.selected)) else {
            return;
        };
        let applied = self.editor.update(cx, |editor, cx| {
            let (text, cursor) = (editor.text(), editor.cursor());
            // Only apply to the link the popup was opened for.
            if wiki_links::link_query(text, cursor).map(|q| q.start) != Some(completion.start) {
                return false;
            }
            let (range, insert) =
                wiki_links::completion_edit(text, completion.start, cursor, &item.label);
            editor.replace_range(range, &insert, cx);
            true
        });
        let handle = self.editor.read(cx).focus_handle(cx);
        window.focus(&handle, cx);
        cx.notify();
        if applied {
            // `replace_range` does not emit `Changed`; record the edit here.
            self.note_edited(window, cx);
            self.mark_broken_links(cx);
        }
    }

    /// Follow a clicked wiki-link. The editor sends the same event for
    /// `[[links]]`, `#tags`, property pills and block refs, and also for
    /// link syntax inside fenced code, so the host looks up what was under
    /// the press and acts only on a real wiki-link.
    pub(crate) fn follow_wiki_link(
        &mut self,
        payload: &SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let press = self.last_press.take();
        let offset = press.and_then(|position| {
            self.editor.update(cx, |editor, cx| {
                let utf16 = editor.character_index_for_point(position, window, cx)?;
                Some(wiki_links::byte_offset_from_utf16(editor.text(), utf16))
            })
        });
        let target = {
            let text = self.editor.read(cx).text();
            let links = wiki_links::wiki_links(text);
            match offset {
                Some(offset) => wiki_links::link_at(&links, offset).map(|link| link.target.clone()),
                None => links
                    .iter()
                    .find(|link| link.target == payload.as_ref())
                    .map(|link| link.target.clone()),
            }
        };
        let Some(target) = target else {
            return;
        };
        self.navigate_to_link(&target, window, cx);
    }

    /// Open the note a wiki-link target resolves to, and scroll to its
    /// heading when the target is an anchor. Broken targets do nothing.
    pub(crate) fn navigate_to_link(
        &mut self,
        target: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let CaveStatus::Open { cave, .. } = &self.status else {
            return;
        };
        let Some(href) = cave.resolve_link(target) else {
            eprintln!("granit-gpui: [[{target}]] does not resolve");
            return;
        };
        // Notes take precedence over anchors, so only a non-note target
        // carries a `#anchor` (and a slug may itself contain `#`).
        let (slug, anchor) = match cave.lookup_slug(target) {
            Some(_) => (href, None),
            None => match href.rsplit_once('#') {
                Some((slug, anchor)) => (slug.to_string(), Some(anchor.to_string())),
                None => (href, None),
            },
        };
        self.open_note(&slug, window, cx);
        if self.open_slug() != Some(slug.as_str()) {
            // The current note could not be saved, so it stays open.
            return;
        }
        self.select_in_tree(&slug, cx);
        if let Some(anchor) = anchor {
            self.pending_anchor =
                wiki_links::anchor_heading_offset(self.editor.read(cx).text(), &anchor);
            self.schedule_after_paint(window, cx);
        }
    }

    /// Open an `[text](url)` link or bare URL in the system browser.
    pub(crate) fn open_external_link(&mut self, url: &SharedString, cx: &mut Context<Self>) {
        if is_safe_external_url(url) {
            cx.open_url(url);
        } else {
            eprintln!("granit-gpui: not opening {url}");
        }
    }

    /// Run [`Self::after_paint`] once the editor has painted the current
    /// state. Next-frame callbacks run before that frame's paint, so this
    /// waits one frame more.
    fn schedule_after_paint(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.on_next_frame(window, |_, window, cx| {
            cx.on_next_frame(window, |this, _, cx| this.after_paint(cx));
        });
    }

    /// Work that needs the editor's fresh layout: scroll to a pending
    /// heading, and re-render the popup at the current caret box.
    pub(crate) fn after_paint(&mut self, cx: &mut Context<Self>) {
        if let Some(offset) = self.pending_anchor.take() {
            self.scroll_offset_to_top(offset, cx);
        }
        cx.notify();
    }

    /// Scroll so the row of byte `offset` sits near the top of the
    /// viewport. Returns whether the editor had painted that row.
    fn scroll_offset_to_top(&mut self, offset: usize, cx: &mut Context<Self>) -> bool {
        let Some(top) = self.editor.read(cx).offset_screen_top(offset) else {
            return false;
        };
        let viewport = self.scroll.bounds();
        let current = self.scroll.offset();
        let y = current.y - (top - ANCHOR_MARGIN - viewport.top());
        // Offsets run from 0 (top) to -max (bottom).
        let y = y.min(px(0.)).max(-self.scroll.max_offset().y);
        self.scroll.set_offset(point(current.x, y));
        true
    }

    /// Capture the editor's navigation keys while the popup is open, so they
    /// move the selection, accept or dismiss instead of editing. Capture
    /// listeners on an ancestor run before the editor's own action handlers.
    pub(crate) fn intercept_completion_keys(&self, area: Div, cx: &mut Context<Self>) -> Div {
        if self.completion.is_none() {
            return area;
        }
        area.capture_action(cx.listener(|this, _: &zorite_editor::Up, _, cx| {
            this.move_completion(-1, cx);
            cx.stop_propagation();
        }))
        .capture_action(cx.listener(|this, _: &zorite_editor::Down, _, cx| {
            this.move_completion(1, cx);
            cx.stop_propagation();
        }))
        .capture_action(cx.listener(|this, _: &zorite_editor::Newline, window, cx| {
            this.accept_completion(None, window, cx);
            cx.stop_propagation();
        }))
        .capture_action(cx.listener(|this, _: &zorite_editor::Indent, window, cx| {
            this.accept_completion(None, window, cx);
            cx.stop_propagation();
        }))
        .capture_action(cx.listener(|this, _: &zorite_editor::Dismiss, _, cx| {
            this.dismiss_completion(cx);
            cx.stop_propagation();
        }))
    }

    /// The popup, anchored below the `[[` it completes (above when there is
    /// no room). Nothing until the editor has painted that offset.
    pub(crate) fn render_completion(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let completion = self.completion.as_ref()?;
        let caret = self.editor.read(cx).bounds_for_offset(completion.start)?;
        let theme = cx.theme();
        let rows = completion.items.iter().enumerate().map(|(ix, item)| {
            let (detail, muted): (Option<SharedString>, bool) = match &item.kind {
                CandidateKind::Note => (None, false),
                CandidateKind::Anchor { note } => (Some(format!("# in {note}").into()), false),
                CandidateKind::Broken => (Some("no note yet".into()), true),
                CandidateKind::Create => (Some("new note".into()), true),
            };
            let selected = ix == completion.selected;
            div()
                .id(ix)
                .flex()
                .flex_row()
                .gap_2()
                .items_center()
                .px_2()
                .py_1()
                .rounded(px(4.))
                .when(selected, |row| {
                    row.bg(theme.accent).text_color(theme.accent_foreground)
                })
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .when(muted, |label| label.italic())
                        .child(SharedString::from(item.label.clone())),
                )
                .children(detail.map(|detail| {
                    div()
                        .flex_none()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(detail)
                }))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, window, cx| {
                        this.accept_completion(Some(ix), window, cx);
                        cx.stop_propagation();
                    }),
                )
        });
        let list = div()
            .id("link-completion")
            .w(px(320.))
            .max_h(px(260.))
            .overflow_y_scroll()
            .track_scroll(&completion.scroll)
            .p_1()
            .bg(theme.popover)
            .text_color(theme.popover_foreground)
            .text_sm()
            .border_1()
            .border_color(theme.border)
            .rounded(px(6.))
            .shadow_md()
            .children(rows);
        Some(
            deferred(
                anchored()
                    .attach(caret, Anchor::BottomLeft)
                    .offset(point(px(0.), px(4.)))
                    .child(list),
            )
            .with_priority(1)
            .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext, Modifiers, TestAppContext, VisualTestContext, WindowHandle};

    use super::*;
    use crate::tests::{editor_text, open, open_workspace};

    const LINKS: &str = "# Links\n\nPlain [[b]] and [[Räksmörgås och kaffe|fika]].\n\
                         Anchor [[volvo]] and [[Missing]].\n`[[b]]` code\n\n\
                         ```\n[[b]]\n```\n\n#tag here\n\nWeb [site](https://example.com/x).\n";

    /// A cave with a link-rich note, a non-ASCII note and a long note whose
    /// anchored heading is far below the fold.
    fn link_cave() -> tempfile::TempDir {
        let dir = crate::tests::fixture_cave();
        std::fs::write(dir.path().join("links.md"), LINKS).unwrap();
        std::fs::write(dir.path().join("Räksmörgås och kaffe.md"), "# Fika\n").unwrap();
        // A note named like the tag, so a tag click that were treated as a
        // wiki-link would navigate.
        std::fs::write(dir.path().join("tag.md"), "# Tag\n").unwrap();
        let filler = "Filler line.\n\n".repeat(150);
        std::fs::write(
            dir.path().join("long.md"),
            format!("# Long\n\n{filler}## Volvo {{#volvo}}\n\n{filler}"),
        )
        .unwrap();
        dir
    }

    fn open_links_workspace(
        cx: &mut TestAppContext,
    ) -> (tempfile::TempDir, WindowHandle<Workspace>) {
        let dir = link_cave();
        let window = open_workspace(dir.path(), cx);
        cx.update(zorite_editor::bind_keys);
        open(window, "links", cx);
        draw(window, cx);
        (dir, window)
    }

    fn draw(window: WindowHandle<Workspace>, cx: &mut TestAppContext) {
        cx.update_window(window.into(), |_, window, cx| {
            let _ = window.draw(cx);
        })
        .unwrap();
    }

    fn open_slug(window: WindowHandle<Workspace>, cx: &mut TestAppContext) -> Option<String> {
        window
            .update(cx, |this, _, _| this.open_slug().map(str::to_string))
            .unwrap()
    }

    /// Click the painted text of byte `offset` in the open note.
    fn click_offset(window: WindowHandle<Workspace>, offset: usize, cx: &mut TestAppContext) {
        let caret = window
            .update(cx, |this, _, cx| {
                this.editor.read(cx).bounds_for_offset(offset)
            })
            .unwrap()
            .expect("the offset was painted");
        let position = point(caret.left() + px(2.), caret.top() + caret.size.height / 2.);
        let mut vcx = VisualTestContext::from_window(window.into(), cx);
        vcx.simulate_mouse_move(position, None, Modifiers::none());
        vcx.simulate_click(position, Modifiers::none());
        cx.run_until_parked();
    }

    fn offset_in(text: &str, needle: &str) -> usize {
        text.find(needle)
            .unwrap_or_else(|| panic!("{needle:?} not found"))
    }

    #[gpui::test]
    fn clicking_wiki_links_opens_the_target_note(cx: &mut TestAppContext) {
        let (dir, window) = open_links_workspace(cx);
        // Inside the label of a non-ASCII, labelled link.
        click_offset(window, offset_in(LINKS, "fika]]"), cx);
        assert_eq!(
            open_slug(window, cx).as_deref(),
            Some("Räksmörgås och kaffe")
        );
        assert_eq!(editor_text(window, cx), "# Fika\n");
        window
            .update(cx, |this, _, cx| {
                let CaveStatus::Open { tree, .. } = &this.status else {
                    panic!("cave open");
                };
                let selected = tree
                    .read(cx)
                    .selected_item()
                    .map(|item| item.id.to_string());
                assert_eq!(selected.as_deref(), Some("note:Räksmörgås och kaffe"));
            })
            .unwrap();

        open(window, "links", cx);
        draw(window, cx);
        click_offset(window, offset_in(LINKS, "[[b]]") + 2, cx);
        assert_eq!(open_slug(window, cx).as_deref(), Some("b"));

        // Nothing was written by navigating.
        assert_eq!(
            std::fs::read_to_string(dir.path().join("links.md")).unwrap(),
            LINKS
        );
    }

    #[gpui::test]
    fn code_tags_and_broken_links_do_not_navigate(cx: &mut TestAppContext) {
        let (_dir, window) = open_links_workspace(cx);
        for needle in ["`[[b]]`", "\n[[b]]\n", "#tag", "[[Missing]]"] {
            click_offset(window, offset_in(LINKS, needle) + 3, cx);
            assert_eq!(open_slug(window, cx).as_deref(), Some("links"), "{needle}");
            draw(window, cx);
        }
    }

    #[gpui::test]
    fn anchor_link_opens_the_note_and_scrolls_to_the_heading(cx: &mut TestAppContext) {
        let (_dir, window) = open_links_workspace(cx);
        click_offset(window, offset_in(LINKS, "[[volvo]]") + 3, cx);
        assert_eq!(open_slug(window, cx).as_deref(), Some("long"));
        // Frames are not scheduled in tests: paint, then run the
        // post-paint step the next-frame callback would run.
        draw(window, cx);
        window
            .update(cx, |this, _, cx| {
                let heading = this.pending_anchor.expect("heading offset pending");
                assert!(this.editor.read(cx).text()[heading..].starts_with("## Volvo"));
                this.after_paint(cx);
            })
            .unwrap();
        draw(window, cx);
        window
            .update(cx, |this, _, cx| {
                let heading =
                    wiki_links::anchor_heading_offset(this.editor.read(cx).text(), "volvo")
                        .unwrap();
                assert!(this.scroll.offset().y < px(0.), "scrolled down");
                let top = this.editor.read(cx).offset_screen_top(heading).unwrap();
                let viewport = this.scroll.bounds();
                assert!(
                    top >= viewport.top() && top < viewport.top() + px(100.),
                    "heading at {top:?}, viewport {viewport:?}"
                );
            })
            .unwrap();
    }

    #[gpui::test]
    fn external_links_open_through_the_platform(cx: &mut TestAppContext) {
        let (_dir, window) = open_links_workspace(cx);
        click_offset(window, offset_in(LINKS, "site]") + 1, cx);
        assert_eq!(cx.opened_url().as_deref(), Some("https://example.com/x"));
        assert_eq!(open_slug(window, cx).as_deref(), Some("links"));
    }

    #[test]
    fn only_web_and_mail_urls_are_opened() {
        assert!(is_safe_external_url("https://example.com"));
        assert!(is_safe_external_url("HTTP://example.com"));
        assert!(is_safe_external_url("mailto:a@b.c"));
        assert!(!is_safe_external_url("file:///etc/passwd"));
        assert!(!is_safe_external_url("javascript:alert(1)"));
        assert!(!is_safe_external_url(" https://x"));
        assert!(!is_safe_external_url("https://"));
    }

    /// Focus the editor at the end of the open note and paint, so typed
    /// input and bound keys reach it.
    fn focus_at_end(window: WindowHandle<Workspace>, cx: &mut TestAppContext) {
        window
            .update(cx, |this, window, cx| {
                this.editor.update(cx, |editor, cx| {
                    let end = editor.text().len();
                    editor.set_cursor(end, cx);
                    editor.focus(window, cx);
                });
            })
            .unwrap();
        cx.run_until_parked();
        draw(window, cx);
    }

    fn completion_labels(
        window: WindowHandle<Workspace>,
        cx: &mut TestAppContext,
    ) -> Option<Vec<String>> {
        window
            .update(cx, |this, _, _| {
                this.completion
                    .as_ref()
                    .map(|c| c.labels().into_iter().map(str::to_string).collect())
            })
            .unwrap()
    }

    fn type_text(window: WindowHandle<Workspace>, text: &str, cx: &mut TestAppContext) {
        VisualTestContext::from_window(window.into(), cx).simulate_input(text);
        draw(window, cx);
    }

    fn keys(window: WindowHandle<Workspace>, keys: &str, cx: &mut TestAppContext) {
        VisualTestContext::from_window(window.into(), cx).simulate_keystrokes(keys);
        draw(window, cx);
    }

    #[gpui::test]
    fn typing_double_bracket_offers_notes_and_anchors(cx: &mut TestAppContext) {
        let (_dir, window) = open_links_workspace(cx);
        focus_at_end(window, cx);
        type_text(window, "[[", cx);
        let labels = completion_labels(window, cx).expect("popup open");
        for expected in [
            "a",
            "b",
            "links",
            "long",
            "Räksmörgås och kaffe",
            "volvo",
            "Missing",
        ] {
            assert!(
                labels.iter().any(|l| l == expected),
                "{expected} in {labels:?}"
            );
        }
        // The popup is anchored at the painted `[[`.
        window
            .update(cx, |this, _, cx| {
                assert!(this.render_completion(cx).is_some())
            })
            .unwrap();

        type_text(window, "räk", cx);
        assert_eq!(
            completion_labels(window, cx).unwrap(),
            ["Räksmörgås och kaffe", "räk"]
        );
        keys(window, "enter", cx);
        assert!(
            completion_labels(window, cx).is_none(),
            "closed after accept"
        );
        let text = editor_text(window, cx);
        assert!(text.ends_with("[[Räksmörgås och kaffe]]"), "{text:?}");
        // The caret sits after the closing brackets: typing continues there.
        type_text(window, "!", cx);
        assert!(editor_text(window, cx).ends_with("kaffe]]!"));
    }

    #[gpui::test]
    fn arrows_select_tab_accepts_and_escape_dismisses(cx: &mut TestAppContext) {
        let (dir, window) = open_links_workspace(cx);
        focus_at_end(window, cx);
        let before = editor_text(window, cx);
        type_text(window, "[[lo", cx);
        assert_eq!(
            completion_labels(window, cx).unwrap(),
            ["long", "volvo", "lo"]
        );
        keys(window, "down", cx);
        let selected = window
            .update(cx, |this, _, _| {
                this.completion.as_ref().unwrap().selected()
            })
            .unwrap();
        assert_eq!(selected, 1);
        // The arrow moved the selection, not the caret.
        assert_eq!(editor_text(window, cx), format!("{before}[[lo"));
        keys(window, "up up", cx);
        keys(window, "down tab", cx);
        assert_eq!(editor_text(window, cx), format!("{before}[[long]]"));

        type_text(window, " [[b", cx);
        assert!(completion_labels(window, cx).is_some());
        keys(window, "escape", cx);
        assert!(completion_labels(window, cx).is_none());
        // Dismissed stays dismissed while typing in the same link; Enter
        // edits again.
        type_text(window, "x", cx);
        assert!(completion_labels(window, cx).is_none());
        keys(window, "enter", cx);
        assert_eq!(editor_text(window, cx), format!("{before}[[long]] [[bx\n"));

        // The accepted completion is saved like typed text.
        cx.executor().advance_clock(crate::AUTOSAVE_DELAY * 2);
        cx.run_until_parked();
        let saved = std::fs::read_to_string(dir.path().join("links.md")).unwrap();
        assert!(saved.ends_with("[[long]] [[bx\n"), "{saved:?}");
    }

    #[gpui::test]
    fn no_completion_inside_code(cx: &mut TestAppContext) {
        let (_dir, window) = open_links_workspace(cx);
        focus_at_end(window, cx);
        // An unclosed backtick is not code yet, so this completes...
        type_text(window, "`[[", cx);
        assert!(completion_labels(window, cx).is_some());
        // ...until the span is closed around the link.
        type_text(window, "x`", cx);
        assert!(completion_labels(window, cx).is_none());
        keys(window, "left", cx);
        assert!(completion_labels(window, cx).is_none());
        keys(window, "end enter", cx);
        type_text(window, "```", cx);
        keys(window, "enter", cx);
        type_text(window, "[[", cx);
        let text = editor_text(window, cx);
        assert!(text.contains("```\n[["), "{text:?}");
        assert!(completion_labels(window, cx).is_none(), "inside a fence");
    }
}

//! Wiki-links in the editor: following clicked links through the cave
//! resolver, opening external URLs, marking broken links, and the `[[`
//! completion popup.
//!
//! The editor crate has no completion API, so the view drives the popup: it
//! watches `Changed` / `SelectionChanged`, finds the unclosed `[[` before
//! the caret, anchors a list at the editor's caret box, and intercepts the
//! editor's own Up/Down/Enter/Tab/Escape actions in the capture phase while
//! the list is open.

use std::rc::Rc;

use gpui::{
    anchored, deferred, div, point, prelude::FluentBuilder, px, Anchor, AnyElement, App, Context,
    Div, EntityInputHandler, Focusable, InteractiveElement, IntoElement, MouseButton,
    ParentElement, ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Window,
};
use gpui_kit::component::ActiveTheme;
use granit_core::markdown::LinkSpans;
#[cfg(test)]
use granit_core::markdown::Markdown;
use log::warn;
use zorite_editor::Diagnostic;

use super::wiki_links::{self, Candidate, CandidateKind};
use super::{NoteView, NoteViewEvent, Reveal};
use crate::cave_ops;
use crate::host::Host;

/// Most entries the popup lists.
const COMPLETION_LIMIT: usize = 50;

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

/// Open an `[text](url)` link or bare URL in the system browser.
pub fn open_external_link(url: &SharedString, cx: &mut App) {
    if is_safe_external_url(url) {
        cx.open_url(url);
    } else {
        warn!("Not opening the link {url}");
    }
}

impl NoteView {
    /// The completion popup, while it is open.
    #[cfg(test)]
    pub(super) fn completion(&self) -> Option<&LinkCompletion> {
        self.completion.as_ref()
    }

    /// Underline the open note's wiki-links whose target does not resolve.
    /// The editor has one link color, so broken links get its diagnostic
    /// squiggle instead.
    pub(super) fn mark_broken_links(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = self.editor.clone() else {
            return;
        };
        // A template's links are filled in when a note is made from it
        // (`[[{{ yesterday }}]]`), so none of them counts as broken.
        if self.template_slug().is_some() {
            editor.update(cx, |editor, cx| editor.set_diagnostics(Vec::new(), cx));
            return;
        }
        let spans = self.link_spans.get(editor.read(cx).text());
        let diagnostics = self
            .links
            .broken_ranges(&spans)
            .into_iter()
            .map(|range| Diagnostic { range })
            .collect();
        editor.update(cx, |editor, cx| editor.set_diagnostics(diagnostics, cx));
    }

    /// The wiki-links, code and anchors of the editor's text.
    fn spans(&mut self, cx: &App) -> Option<Rc<LinkSpans>> {
        let editor = self.editor.clone()?;
        Some(self.link_spans.get(editor.read(cx).text()))
    }

    /// The byte ranges of the broken links marked in the open note.
    #[cfg(test)]
    pub(super) fn broken_links(&self, cx: &App) -> Vec<std::ops::Range<usize>> {
        let Some(text) = self.text(cx) else {
            return Vec::new();
        };
        if self.template_slug().is_some() {
            return Vec::new();
        }
        self.links.broken_ranges(&Markdown::link_spans(text))
    }

    /// Open, refilter or close the popup for the caret's position. Only a
    /// text change (`typed`) opens it; a caret move keeps it in step or
    /// closes it.
    pub(super) fn update_completion(
        &mut self,
        typed: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let query = self.spans(cx).and_then(|spans| {
            let editor = self.editor.as_ref()?.read(cx);
            wiki_links::link_query(editor.text(), &spans, editor.cursor())
        });
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
        let items = self.links.complete(&query.query, COMPLETION_LIMIT);
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
    pub(super) fn accept_completion(
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
        let Some(editor) = self.editor.clone() else {
            return;
        };
        let Some(spans) = self.spans(cx) else {
            return;
        };
        let applied = editor.update(cx, |editor, cx| {
            let (text, cursor) = (editor.text(), editor.cursor());
            // Only apply to the link the popup was opened for.
            if wiki_links::link_query(text, &spans, cursor).map(|q| q.start)
                != Some(completion.start)
            {
                return false;
            }
            let (range, insert) =
                wiki_links::completion_edit(text, completion.start, cursor, &item.label);
            editor.replace_range(range, &insert, cx);
            true
        });
        let handle = editor.read(cx).focus_handle(cx);
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
    /// link syntax inside fenced code, so the view looks up what was under
    /// the press and acts only on a real wiki-link.
    pub(super) fn follow_wiki_link(
        &mut self,
        payload: &SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self.editor.clone() else {
            return;
        };
        let press = self.last_press.take();
        let offset = press.and_then(|position| {
            editor.update(cx, |editor, cx| {
                let utf16 = editor.character_index_for_point(position, window, cx)?;
                Some(wiki_links::byte_offset_from_utf16(editor.text(), utf16))
            })
        });
        let Some(spans) = self.spans(cx) else {
            return;
        };
        let target = match offset {
            Some(offset) => spans.link_at(offset),
            None => spans
                .links
                .iter()
                .find(|link| link.target == payload.as_ref()),
        }
        .map(|link| link.target.clone());
        if let Some(target) = target {
            self.navigate_to_link(&target, cx);
        }
    }

    /// Ask the workspace to open the note `target` resolves to, at its
    /// heading when it is an anchor. A broken target creates the note (in
    /// the cave root, as the web app does) and opens it; in a template it
    /// does nothing, since its targets may be Tera placeholders.
    pub(super) fn navigate_to_link(&mut self, target: &str, cx: &mut Context<Self>) {
        let in_template = self.template_slug().is_some();
        let resolved = Host::core(cx).with_cave(|cave| {
            let Some(href) = cave.resolve_link(target) else {
                return Ok(None);
            };
            // Notes take precedence over anchors, so only a non-note target
            // carries a `#anchor` (and a slug may itself contain `#`).
            Ok(Some(match cave.lookup_slug(target) {
                Some(_) => (href, None),
                None => match href.rsplit_once('#') {
                    Some((slug, anchor)) => (slug.to_string(), Some(anchor.to_string())),
                    None => (href, None),
                },
            }))
        });
        let (slug, anchor) = match resolved {
            Ok(Some(found)) => found,
            Ok(None) if in_template => return,
            Ok(None) => match cave_ops::create_linked_note(target, cx) {
                Ok(meta) => (meta.slug, None),
                Err(err) => {
                    warn!("Could not create the note [[{target}]]: {err}");
                    return;
                }
            },
            Err(err) => {
                warn!("Could not resolve [[{target}]]: {err}");
                return;
            }
        };
        cx.emit(NoteViewEvent::OpenNote {
            slug,
            reveal: anchor.map(Reveal::Anchor),
        });
    }

    /// Capture the editor's navigation keys while the popup is open, so they
    /// move the selection, accept or dismiss instead of editing. Capture
    /// listeners on an ancestor run before the editor's own action handlers.
    pub(super) fn intercept_completion_keys(&self, area: Div, cx: &mut Context<Self>) -> Div {
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
    pub(super) fn render_completion(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let completion = self.completion.as_ref()?;
        let caret = self
            .editor
            .as_ref()?
            .read(cx)
            .bounds_for_offset(completion.start)?;
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
    use super::is_safe_external_url;

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
}

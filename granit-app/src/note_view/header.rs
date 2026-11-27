//! The note header above the editor, as the Tauri app's note header has it:
//! the icon picker, the favorite star (notes only) and the title, which a
//! click turns into a rename input; the tag pills with an "Add tag" input;
//! the created/modified dates, word count and reading time; and, for notes,
//! the foldable backlinks section.
//!
//! What it shows is the core's `Cave::note_header` / `Cave::template_header`
//! of the file on disk, re-read whenever the note is loaded, saved or the
//! cave changes. Every edit changes one owned frontmatter key through
//! `cave_ops::edit_note_header` / `edit_template_header`, after the editor's
//! pending edits are written, so a header edit never drops typed text and
//! the body on disk is the editor's. A title edit renames through
//! `cave_ops::rename_note` / `rename_template`; pending edits follow the
//! document to its new name.

use gpui::{
    div, prelude::FluentBuilder as _, px, AnyElement, AppContext as _, Context, Entity, FontWeight,
    InteractiveElement as _, IntoElement, KeyDownEvent, ParentElement, SharedString,
    StatefulInteractiveElement as _, Styled, Subscription, Window,
};
use gpui_kit::component::{
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputEvent, InputState},
    popover::Popover,
    v_flex, ActiveTheme, Icon, IconName, Sizable,
};
use granit_core::cave::{DocumentHeader, HeaderEdit};
use granit_types::{search_note_icons, NoteIconEntry};
use log::warn;

use super::save::{NoteSession, SaveStatus};
use super::{DocKind, DocRef, FlushOutcome, Note, NoteView, NoteViewEvent};
use crate::assets::note_icon_path;
use crate::cave_ops;

/// The header's state, part of [`NoteView`].
pub(super) struct Header {
    /// What the header shows, from the file on disk.
    info: Option<DocumentHeader>,
    /// The last failed header edit.
    error: Option<SharedString>,
    /// The title being edited.
    rename: Option<TextEdit>,
    /// The "Add tag" input; it lives as long as the view.
    tag_input: Entity<InputState>,
    /// The open icon picker, with its search input.
    icon_picker: Option<TextEdit>,
    /// Whether the backlinks section is folded out.
    backlinks_open: bool,
    _tag_events: Subscription,
}

/// A text input shown while it is in use, with its event subscription.
struct TextEdit {
    input: Entity<InputState>,
    _events: Subscription,
}

impl Header {
    pub(super) fn new(window: &mut Window, cx: &mut Context<NoteView>) -> Self {
        let tag_input = cx.new(|cx| InputState::new(window, cx).placeholder("Add tag…"));
        let events = cx.subscribe_in(
            &tag_input,
            window,
            |this, input, event: &InputEvent, window, cx| match event {
                InputEvent::PressEnter { .. } => this.commit_tags(true, window, cx),
                // A typed comma ends a tag, as in the Tauri app.
                InputEvent::Change if input.read(cx).value().contains(',') => {
                    this.commit_tags(false, window, cx)
                }
                _ => {}
            },
        );
        Self {
            info: None,
            error: None,
            rename: None,
            tag_input,
            icon_picker: None,
            backlinks_open: false,
            _tag_events: events,
        }
    }

    /// Forget the per-document state when another document is loaded.
    pub(super) fn reset(&mut self) {
        self.info = None;
        self.error = None;
        self.rename = None;
        self.icon_picker = None;
        self.backlinks_open = false;
    }

    /// Empty the "Add tag" input (another document was loaded).
    pub(super) fn clear_tag_input(&self, window: &mut Window, cx: &mut Context<NoteView>) {
        if !self.tag_input.read(cx).value().is_empty() {
            self.tag_input
                .update(cx, |input, cx| input.set_value("", window, cx));
        }
    }
}

impl NoteView {
    /// The open document, when it loaded.
    fn loaded_doc(&self) -> Option<DocRef> {
        match &self.note {
            Note::Loaded(loaded) => Some(DocRef {
                kind: loaded.session.kind(),
                slug: loaded.session.slug().to_string(),
            }),
            _ => None,
        }
    }

    /// Re-read what the header shows from the file on disk.
    pub(super) fn refresh_header(&mut self, cx: &mut Context<Self>) {
        let info = self.loaded_doc().and_then(|doc| {
            let header = match doc.kind {
                DocKind::Note => cave_ops::note_header(&doc.slug, cx),
                DocKind::Template => cave_ops::template_header(&doc.slug, cx),
            };
            header
                .inspect_err(|err| warn!("Could not read the header of {}: {err}", doc.slug))
                .ok()
        });
        self.header.info = info;
        cx.notify();
    }

    /// What the header shows, for tests.
    #[cfg(test)]
    pub(super) fn header_info(&self) -> Option<&DocumentHeader> {
        self.header.info.as_ref()
    }

    /// The last failed header edit, for tests.
    #[cfg(test)]
    pub(super) fn header_error(&self) -> Option<&SharedString> {
        self.header.error.as_ref()
    }

    /// Apply `edit` to the open document's frontmatter, after writing its
    /// pending edits (if they cannot be written, nothing is changed and the
    /// error is shown).
    pub fn edit_header(&mut self, edit: HeaderEdit, cx: &mut Context<Self>) {
        let Some(doc) = self.loaded_doc() else {
            return;
        };
        if let FlushOutcome::Failed(err) = self.flush(cx) {
            self.header.error =
                Some(format!("The edits to {} could not be saved: {err}", doc.slug).into());
            cx.notify();
            return;
        }
        let result = match doc.kind {
            DocKind::Note => cave_ops::edit_note_header(&doc.slug, edit, cx),
            DocKind::Template => cave_ops::edit_template_header(&doc.slug, edit, cx),
        };
        self.header.error = result.err().map(|err| {
            warn!("Could not update {}: {err}", doc.slug);
            format!("Could not update {}: {err}", doc.slug).into()
        });
        self.refresh_header(cx);
    }

    /// Star or unstar the open note.
    pub fn toggle_favorite(&mut self, cx: &mut Context<Self>) {
        let favorite = self.header.info.as_ref().is_some_and(|info| info.favorite);
        self.edit_header(HeaderEdit::Favorite(!favorite), cx);
    }

    /// Add the tags typed into the "Add tag" input: everything when `all`
    /// (Enter), else what precedes the last comma, keeping the rest typed.
    fn commit_tags(&mut self, all: bool, window: &mut Window, cx: &mut Context<Self>) {
        let value = self.header.tag_input.read(cx).value().to_string();
        let (done, rest) = match value.rsplit_once(',') {
            _ if all => (value.as_str(), ""),
            Some((done, rest)) => (done, rest.trim_start()),
            None => return,
        };
        let tags: Vec<String> = done.split(',').map(str::to_string).collect();
        let rest = rest.to_string();
        self.header
            .tag_input
            .update(cx, |input, cx| input.set_value(rest, window, cx));
        for tag in tags {
            if granit_core::cave::normalize_tag(&tag).is_some() {
                self.edit_header(HeaderEdit::AddTag(tag), cx);
            }
        }
    }

    /// Type `text` into the "Add tag" input, for tests; `enter` then
    /// presses Enter.
    #[cfg(test)]
    pub(super) fn type_tag(
        &mut self,
        text: &str,
        enter: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = text.to_string();
        self.header
            .tag_input
            .update(cx, |input, cx| input.set_value(text, window, cx));
        if self.header.tag_input.read(cx).value().contains(',') {
            self.commit_tags(false, window, cx);
        }
        if enter {
            self.commit_tags(true, window, cx);
        }
    }

    /// What the "Add tag" input holds, for tests.
    #[cfg(test)]
    pub(super) fn tag_input_text(&self, cx: &gpui::App) -> String {
        self.header.tag_input.read(cx).value().to_string()
    }

    /// Turn the title into a text input holding the name. Enter or leaving
    /// the input renames, Escape cancels.
    pub fn start_title_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(doc) = self.loaded_doc() else {
            return;
        };
        let input = cx.new(|cx| InputState::new(window, cx).default_value(doc.slug));
        let events = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                this.commit_title(cx);
            }
        });
        input.update(cx, |input, cx| {
            input.focus(window, cx);
            input.select_all(window, cx);
        });
        self.header.rename = Some(TextEdit {
            input,
            _events: events,
        });
        cx.notify();
    }

    /// Stop editing the title without renaming.
    pub fn cancel_title_edit(&mut self, cx: &mut Context<Self>) {
        self.header.rename = None;
        cx.notify();
    }

    /// Rename the open document to the typed title; an empty or unchanged
    /// title renames nothing.
    pub fn commit_title(&mut self, cx: &mut Context<Self>) {
        let Some(edit) = self.header.rename.take() else {
            return;
        };
        cx.notify();
        let name = edit.input.read(cx).value().trim().to_string();
        let Some(doc) = self.loaded_doc() else {
            return;
        };
        if name.is_empty() || name == doc.slug {
            return;
        }
        let result = match doc.kind {
            DocKind::Note => cave_ops::rename_note(&doc.slug, &name, cx).map(|_| ()),
            DocKind::Template => cave_ops::rename_template(&doc.slug, &name, cx).map(|_| ()),
        };
        self.header.error = result.err().map(|err| {
            warn!("Could not rename {}: {err}", doc.slug);
            format!("Could not rename {}: {err}", doc.slug).into()
        });
    }

    /// Type `name` into the open title input, for tests.
    #[cfg(test)]
    pub(super) fn type_title(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(edit) = &self.header.rename {
            let name = name.to_string();
            edit.input
                .update(cx, |input, cx| input.set_value(name, window, cx));
        }
    }

    /// Open the icon picker with its search field focused.
    pub fn open_icon_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.header.icon_picker.is_some() || self.loaded_doc().is_none() {
            return;
        }
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Search icons…"));
        let events = cx.subscribe(&input, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        input.update(cx, |input, cx| input.focus(window, cx));
        self.header.icon_picker = Some(TextEdit {
            input,
            _events: events,
        });
        cx.notify();
    }

    pub fn close_icon_picker(&mut self, cx: &mut Context<Self>) {
        if self.header.icon_picker.take().is_some() {
            cx.notify();
        }
    }

    /// Set (or, with `None`, remove) the open document's icon and close
    /// the picker.
    pub fn choose_icon(&mut self, icon: Option<String>, cx: &mut Context<Self>) {
        self.close_icon_picker(cx);
        self.edit_header(HeaderEdit::Icon(icon), cx);
    }

    /// The icons the open picker offers for its search.
    fn icon_choices(&self, cx: &gpui::App) -> Option<Vec<&'static NoteIconEntry>> {
        let picker = self.header.icon_picker.as_ref()?;
        Some(search_note_icons(&picker.input.read(cx).value()))
    }

    /// Type `query` into the open icon search, for tests, and return the
    /// ids offered.
    #[cfg(test)]
    pub(super) fn search_icons(
        &mut self,
        query: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Vec<&'static str>> {
        let picker = self.header.icon_picker.as_ref()?;
        let query = query.to_string();
        picker
            .input
            .update(cx, |input, cx| input.set_value(query, window, cx));
        Some(
            self.icon_choices(cx)?
                .into_iter()
                .map(|entry| entry.id)
                .collect(),
        )
    }

    #[cfg(test)]
    pub(super) fn icon_picker_open(&self) -> bool {
        self.header.icon_picker.is_some()
    }

    /// Fold the backlinks section in or out.
    pub fn toggle_backlinks(&mut self, cx: &mut Context<Self>) {
        self.header.backlinks_open = !self.header.backlinks_open;
        cx.notify();
    }

    /// Ask the workspace to open the backlink `slug`.
    pub fn open_backlink(&mut self, slug: String, cx: &mut Context<Self>) {
        cx.emit(NoteViewEvent::OpenNote { slug, reveal: None });
    }

    /// The header: title row, tags, dates and backlinks.
    pub(super) fn render_header(
        &self,
        session: &NoteSession,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_note = session.kind() == DocKind::Note;
        let title_row = self.render_title_row(session, cx);
        let tags = self.render_tags(cx);
        let stats = self.render_stats(cx);
        let backlinks = is_note.then(|| self.render_backlinks(cx)).flatten();
        let theme = cx.theme();
        let error = self.header.error.clone().map(|error| {
            div()
                .text_xs()
                .text_color(theme.danger)
                .child(error)
                .into_any_element()
        });
        v_flex()
            .flex_none()
            .gap_1()
            .px_6()
            .pt_3()
            .pb_2()
            .border_b_1()
            .border_color(theme.border)
            .child(title_row)
            .child(tags)
            .children(stats)
            .children(backlinks)
            .children(error)
    }

    /// Template badge, icon picker, favorite star, title (or its rename
    /// input) and the save state.
    fn render_title_row(&self, session: &NoteSession, cx: &mut Context<Self>) -> AnyElement {
        let is_note = session.kind() == DocKind::Note;
        let icon_picker = self.render_icon_picker(cx);
        let theme = cx.theme();
        let (status, status_color): (SharedString, _) = match session.status() {
            SaveStatus::Clean => ("".into(), theme.muted_foreground),
            SaveStatus::Edited => ("Edited".into(), theme.muted_foreground),
            SaveStatus::Failed(err) => (format!("Save failed: {err}").into(), theme.danger),
        };
        let favorite = self.header.info.as_ref().is_some_and(|info| info.favorite);
        let star = is_note.then(|| {
            Button::new("note-favorite")
                .ghost()
                .small()
                .icon(Icon::new(IconName::Star).text_color(if favorite {
                    theme.warning
                } else {
                    theme.muted_foreground
                }))
                .tooltip(if favorite {
                    "Unfavorite note"
                } else {
                    "Favorite note"
                })
                .on_click(cx.listener(|this, _, _, cx| this.toggle_favorite(cx)))
        });
        let title = match &self.header.rename {
            Some(edit) => div()
                .flex_1()
                .min_w_0()
                .capture_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                    if event.keystroke.key == "escape" {
                        this.cancel_title_edit(cx);
                    }
                }))
                .child(Input::new(&edit.input))
                .into_any_element(),
            None => div()
                .id("note-title")
                .flex_1()
                .min_w_0()
                .truncate()
                .text_xl()
                .font_weight(FontWeight::BOLD)
                .cursor_text()
                .on_click(cx.listener(|this, _, window, cx| this.start_title_edit(window, cx)))
                .child(SharedString::from(session.slug().to_string()))
                .into_any_element(),
        };
        h_flex()
            .gap_2()
            .when(!is_note, |row| {
                row.child(
                    div()
                        .flex_none()
                        .px_2()
                        .rounded(px(4.))
                        .text_xs()
                        .bg(theme.muted)
                        .text_color(theme.muted_foreground)
                        .child("Template"),
                )
            })
            .child(icon_picker)
            .children(star)
            .child(title)
            .child(
                div()
                    .flex_none()
                    .text_sm()
                    .text_color(status_color)
                    .child(status),
            )
            .into_any_element()
    }

    /// The icon button and, while open, the searchable grid of
    /// [`granit_types::NOTE_ICONS`].
    fn render_icon_picker(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let current = self.header.info.as_ref().and_then(|info| info.icon.clone());
        let trigger = Button::new("note-icon")
            .ghost()
            .small()
            .icon(
                Icon::default()
                    .path(note_icon_path(current.as_deref()))
                    .text_color(if current.is_some() {
                        theme.primary
                    } else {
                        theme.muted_foreground
                    }),
            )
            .tooltip("Change icon");
        let view = cx.weak_entity();
        let on_change = {
            let view = view.clone();
            move |open: &bool, window: &mut Window, cx: &mut gpui::App| {
                let open = *open;
                view.update(cx, |this, cx| {
                    if open {
                        this.open_icon_picker(window, cx);
                    } else {
                        this.close_icon_picker(cx);
                    }
                })
                .ok();
            }
        };
        let popover = Popover::new("note-icon-picker")
            .trigger(trigger)
            .open(self.header.icon_picker.is_some())
            .on_open_change(on_change);
        // The selector lets tests find the trigger to click it.
        let wrap = |popover: Popover| {
            div()
                .flex_none()
                .debug_selector(|| "note-icon-trigger".into())
                .child(popover)
                .into_any_element()
        };
        let (Some(picker), Some(choices)) = (&self.header.icon_picker, self.icon_choices(cx))
        else {
            return wrap(popover);
        };
        let search = picker.input.clone();
        let primary = theme.primary;
        wrap(popover.content(move |_, _, _| {
            let buttons = choices.iter().enumerate().map(|(ix, entry)| {
                let selected = current.as_deref() == Some(entry.id);
                let view = view.clone();
                let id = entry.id;
                Button::new(("note-icon-choice", ix))
                    .map(|button| {
                        if selected {
                            button.primary()
                        } else {
                            button.ghost()
                        }
                    })
                    .small()
                    .icon(
                        Icon::default()
                            .path(note_icon_path(Some(id)))
                            .when(!selected, |icon| icon.text_color(primary)),
                    )
                    .tooltip(entry.label)
                    .on_click(move |_, _, cx| {
                        view.update(cx, |this, cx| this.choose_icon(Some(id.to_string()), cx))
                            .ok();
                    })
            });
            let remove = current.is_some().then(|| {
                let view = view.clone();
                Button::new("note-icon-remove")
                    .ghost()
                    .xsmall()
                    .label("Remove icon")
                    .on_click(move |_, _, cx| {
                        view.update(cx, |this, cx| this.choose_icon(None, cx)).ok();
                    })
            });
            v_flex()
                .w(px(288.))
                .gap_2()
                .child(Input::new(&search).small())
                .child(
                    div()
                        .id("note-icon-grid")
                        .debug_selector(|| "note-icon-grid".into())
                        .max_h(px(192.))
                        .overflow_y_scroll()
                        .child(h_flex().flex_wrap().gap_1().children(buttons)),
                )
                .children(remove)
        }))
    }

    /// The tag pills, each with a remove button, and the "Add tag" input.
    fn render_tags(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let tags = self
            .header
            .info
            .as_ref()
            .map(|info| info.tags.clone())
            .unwrap_or_default();
        let pills: Vec<AnyElement> = tags
            .into_iter()
            .enumerate()
            .map(|(ix, tag)| {
                let remove = tag.clone();
                h_flex()
                    .flex_none()
                    .gap_1()
                    .pl_2()
                    .rounded(px(4.))
                    .text_xs()
                    .bg(theme.muted)
                    .text_color(theme.muted_foreground)
                    .child(SharedString::from(tag))
                    .child(
                        Button::new(("remove-tag", ix))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Close)
                            .tooltip("Remove tag")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.edit_header(HeaderEdit::RemoveTag(remove.clone()), cx)
                            })),
                    )
                    .into_any_element()
            })
            .collect();
        h_flex()
            .flex_wrap()
            .gap_1()
            .children(pills)
            .child(
                div()
                    .w(px(140.))
                    .child(Input::new(&self.header.tag_input).xsmall()),
            )
            .into_any_element()
    }

    /// Created and modified dates, word count and reading time, as the
    /// Tauri reader shows them under the title.
    fn render_stats(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let stats = &self.header.info.as_ref()?.stats;
        let mut parts: Vec<String> = Vec::new();
        if let Some(created) = &stats.created_display {
            parts.push(format!("Created: {created}"));
        }
        if let Some(modified) = &stats.modified_display {
            parts.push(format!("Modified: {modified}"));
        }
        if stats.word_count > 0 {
            let words = if stats.word_count == 1 {
                "word"
            } else {
                "words"
            };
            parts.push(format!(
                "{} {words} · ~{} min read",
                stats.word_count, stats.reading_minutes
            ));
        }
        if parts.is_empty() {
            return None;
        }
        let theme = cx.theme();
        Some(
            h_flex()
                .flex_wrap()
                .gap_x_4()
                .text_xs()
                .italic()
                .text_color(theme.muted_foreground)
                .children(parts.into_iter().map(SharedString::from))
                .into_any_element(),
        )
    }

    /// "Backlinks (n)", folding out the notes that link here.
    fn render_backlinks(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let backlinks = &self.header.info.as_ref()?.backlinks;
        if backlinks.is_empty() {
            return None;
        }
        let open = self.header.backlinks_open;
        let theme = cx.theme();
        let rows: Vec<AnyElement> = if open {
            backlinks
                .iter()
                .enumerate()
                .map(|(ix, meta)| {
                    let slug = meta.slug.clone();
                    h_flex()
                        .id(("backlink", ix))
                        .gap_2()
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .cursor_pointer()
                        .hover(|row| row.bg(theme.muted))
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.open_backlink(slug.clone(), cx)),
                        )
                        .child(
                            Icon::default()
                                .path(note_icon_path(meta.icon.as_deref()))
                                .small()
                                .text_color(theme.primary),
                        )
                        .child(
                            v_flex()
                                .min_w_0()
                                .child(
                                    div()
                                        .truncate()
                                        .text_sm()
                                        .child(SharedString::from(meta.slug.clone())),
                                )
                                .child(
                                    div()
                                        .truncate()
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(SharedString::from(meta.relative_path.clone())),
                                ),
                        )
                        .into_any_element()
                })
                .collect()
        } else {
            Vec::new()
        };
        Some(
            v_flex()
                .child(
                    h_flex()
                        .id("backlinks-toggle")
                        .gap_1()
                        .cursor_pointer()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .on_click(cx.listener(|this, _, _, cx| this.toggle_backlinks(cx)))
                        .child(
                            Icon::new(if open {
                                IconName::ChevronDown
                            } else {
                                IconName::ChevronRight
                            })
                            .xsmall(),
                        )
                        .child(SharedString::from(format!(
                            "Backlinks ({})",
                            backlinks.len()
                        ))),
                )
                .when(open, |section| {
                    section.child(
                        div()
                            .id("backlinks-list")
                            .max_h(px(200.))
                            .overflow_y_scroll()
                            .children(rows),
                    )
                })
                .into_any_element(),
        )
    }
}

//! The main area: a read-only placeholder showing the open note's body.
//!
//! The editor task (`zem.4k`) replaces this view with the live-preview
//! editor. The workspace drives it through [`NoteView::open`] and
//! [`NoteView::reload`] and keeps no other contract with it.

use gpui::{
    div, prelude::FluentBuilder as _, Context, FontWeight, InteractiveElement as _, IntoElement,
    ParentElement, Render, SharedString, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_kit::component::ActiveTheme;

use crate::cave_ops;

/// What the main area shows.
enum Content {
    /// No note is open.
    Empty,
    /// The note `slug` with its body (frontmatter is never shown).
    Note { slug: String, body: SharedString },
    /// The note could not be read.
    Failed { slug: String, message: SharedString },
}

pub struct NoteView {
    content: Content,
}

impl NoteView {
    pub fn new() -> Self {
        Self {
            content: Content::Empty,
        }
    }

    /// The slug of the shown note.
    pub fn slug(&self) -> Option<&str> {
        match &self.content {
            Content::Empty => None,
            Content::Note { slug, .. } | Content::Failed { slug, .. } => Some(slug),
        }
    }

    /// The shown body, while a note is shown.
    #[cfg(test)]
    pub fn body(&self) -> Option<&str> {
        match &self.content {
            Content::Note { body, .. } => Some(body),
            _ => None,
        }
    }

    /// Show the note `slug`, or nothing.
    pub fn open(&mut self, slug: Option<&str>, cx: &mut Context<Self>) {
        self.content = match slug {
            None => Content::Empty,
            Some(slug) => match cave_ops::read_note(slug, cx) {
                Ok(doc) => Content::Note {
                    slug: slug.to_string(),
                    body: doc.content.into(),
                },
                Err(err) => Content::Failed {
                    slug: slug.to_string(),
                    message: format!("Could not open {slug}: {err}").into(),
                },
            },
        };
        cx.notify();
    }

    /// Re-read the shown note after a cave change.
    pub fn reload(&mut self, cx: &mut Context<Self>) {
        let slug = self.slug().map(str::to_string);
        self.open(slug.as_deref(), cx);
    }
}

impl Default for NoteView {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for NoteView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let container = div().flex_1().min_w_0().h_full().flex().flex_col();
        match &self.content {
            Content::Empty => container.items_center().justify_center().child(
                div()
                    .text_color(theme.muted_foreground)
                    .child("Select a note in the explorer"),
            ),
            Content::Failed { message, .. } => container
                .items_center()
                .justify_center()
                .p_6()
                .child(div().text_color(theme.danger).child(message.clone())),
            Content::Note { slug, body } => container
                .child(
                    div()
                        .flex_none()
                        .px_6()
                        .py_2()
                        .border_b_1()
                        .border_color(theme.border)
                        .font_weight(FontWeight::SEMIBOLD)
                        .truncate()
                        .child(SharedString::from(slug.clone())),
                )
                .child(
                    div()
                        .id("note-body")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .px_8()
                        .py_6()
                        .when(body.is_empty(), |this| {
                            this.text_color(theme.muted_foreground).child("Empty note")
                        })
                        .when(!body.is_empty(), |this| {
                            this.child(div().max_w(gpui::px(820.)).child(body.clone()))
                        }),
                ),
        }
    }
}

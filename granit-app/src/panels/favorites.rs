//! The favorites panel: quick access to the notes starred with the
//! `favorite` frontmatter field, as the Tauri app's Favorites tab lists
//! them. A click opens the note; the open note is highlighted.
//!
//! The list is [`cave_ops::list_favorites`], reread after every cave change
//! while the panel is shown (a hidden panel is marked stale and rereads it
//! once shown again).

use gpui::{
    div, prelude::FluentBuilder as _, Context, EventEmitter, InteractiveElement as _, IntoElement,
    ParentElement, Render, SharedString, StatefulInteractiveElement as _, Styled, Subscription,
    Window,
};
use gpui_kit::component::{h_flex, v_flex, ActiveTheme, Icon, IconName, Sizable};
use granit_core::cave::{CaveError, DocumentMeta};
use log::warn;

use super::{empty_message, error_line, PanelEvent, ROW_HEIGHT};
use crate::assets::note_icon_path;
use crate::cave_ops;
use crate::events::CaveEvents;

pub struct FavoritesPanel {
    favorites: Vec<DocumentMeta>,
    /// The note open in the main area.
    active: Option<String>,
    /// Whether the panel is the sidebar's shown tab.
    visible: bool,
    /// A cave change happened while hidden: reread the list once shown.
    stale: bool,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<PanelEvent> for FavoritesPanel {}

impl FavoritesPanel {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![cx.subscribe(&CaveEvents::entity(cx), |this, _, _, cx| {
            if this.visible {
                this.refresh(cx);
            } else {
                this.stale = true;
            }
        })];
        Self {
            favorites: Vec::new(),
            active: None,
            visible: false,
            stale: true,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    /// The panel was shown or hidden; a stale panel refreshes when shown.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        self.visible = visible;
        if visible && self.stale {
            self.refresh(cx);
        }
    }

    /// The note open in the main area changed.
    pub fn set_active(&mut self, slug: Option<String>, cx: &mut Context<Self>) {
        self.active = slug;
        cx.notify();
    }

    /// Reread the favorite notes.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.stale = false;
        match cave_ops::list_favorites(cx) {
            Ok(favorites) => {
                self.favorites = favorites;
                self.error = None;
            }
            Err(CaveError::NoCaveOpen) => self.favorites.clear(),
            Err(err) => {
                warn!("Could not list the favorites: {err}");
                self.error = Some(format!("Could not list the favorites: {err}").into());
            }
        }
        cx.notify();
    }

    /// Ask the workspace to open the note `slug`.
    pub fn open(&mut self, slug: String, cx: &mut Context<Self>) {
        cx.emit(PanelEvent::OpenNote { slug, reveal: None });
    }

    /// The listed slugs, for tests.
    #[cfg(test)]
    pub fn slugs(&self) -> Vec<String> {
        self.favorites
            .iter()
            .map(|meta| meta.slug.clone())
            .collect()
    }

    /// The note shown as open, for tests.
    #[cfg(test)]
    pub fn active(&self) -> Option<&str> {
        self.active.as_deref()
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        h_flex()
            .flex_none()
            .px_3()
            .py_1()
            .gap_1()
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .truncate()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("Favorites"),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_xs()
                            .text_color(theme.muted_foreground.opacity(0.7))
                            .child("Quick access to starred notes"),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .px_1p5()
                    .rounded_md()
                    .text_xs()
                    .bg(theme.warning.opacity(0.15))
                    .text_color(theme.warning)
                    .child(self.favorites.len().to_string()),
            )
    }

    fn render_row(
        &self,
        ix: usize,
        meta: &DocumentMeta,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = cx.theme();
        let slug = meta.slug.clone();
        let active = self.active.as_deref() == Some(slug.as_str());
        let open_slug = slug.clone();
        h_flex()
            .id(("favorite", ix))
            .h(ROW_HEIGHT)
            .px_2()
            .gap_1()
            .rounded_md()
            .text_sm()
            .cursor_pointer()
            .when(active, |row| {
                row.bg(theme.accent).text_color(theme.accent_foreground)
            })
            .when(!active, |row| row.hover(|row| row.bg(theme.muted)))
            .child(
                Icon::default()
                    .path(note_icon_path(meta.icon.as_deref()))
                    .small()
                    .text_color(theme.primary),
            )
            .child(Icon::new(IconName::Star).xsmall().text_color(theme.warning))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .child(SharedString::from(slug)),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.open(open_slug.clone(), cx)))
            .into_any_element()
    }
}

impl Render for FavoritesPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows: Vec<gpui::AnyElement> = self
            .favorites
            .clone()
            .iter()
            .enumerate()
            .map(|(ix, meta)| self.render_row(ix, meta, cx))
            .collect();
        let list = if rows.is_empty() {
            empty_message("No favorite notes yet", cx).into_any_element()
        } else {
            div()
                .id("favorites-list")
                .size_full()
                .overflow_y_scroll()
                .px_1()
                .children(rows)
                .into_any_element()
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(self.render_header(cx))
            .child(div().flex_1().min_h_0().child(list))
            .children(error_line(self.error.clone(), cx))
    }
}

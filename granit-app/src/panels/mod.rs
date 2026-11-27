//! The sidebar panels that browse the cave beside the explorer: full-text
//! search, the todos of every note, the tags, and the calendar of daily
//! notes.
//!
//! The sidebar shows the calendar above a tab bar ([`SidebarTab`]) that
//! switches between the explorer and the [`SearchPanel`], [`TodosPanel`] and
//! [`TagsPanel`], as the Tauri app does. Each panel asks the workspace to
//! open a note with [`PanelEvent::OpenNote`] (with the spot to reveal for a
//! search or todo match); the todos panel asks it to toggle a todo with
//! [`PanelEvent::ToggleTodo`], so the open note's pending edits are written
//! first. The queries are the core's; the list panels run them on the
//! background executor, and only while shown: a cave change marks a hidden
//! panel stale and it reruns its query once shown again.

pub mod calendar;
pub mod search;
pub mod tags;
pub mod todos;

#[cfg(test)]
mod tests;

use gpui::{div, px, App, IntoElement, ParentElement as _, Pixels, SharedString, Styled as _};
use gpui_kit::component::{ActiveTheme as _, Icon, IconName};
use granit_types::TodoItem;

use crate::assets::AppIcon;
use crate::note_view::Reveal;

pub use calendar::CalendarPanel;
pub use search::SearchPanel;
pub use tags::TagsPanel;
pub use todos::TodosPanel;

/// What a panel asks of the workspace.
#[derive(Clone, Debug)]
pub enum PanelEvent {
    /// Open `slug`, then reveal `reveal` in it.
    OpenNote {
        slug: String,
        reveal: Option<Reveal>,
    },
    /// Toggle this todo in its note; `checked` is its state as listed.
    ToggleTodo { item: TodoItem, checked: bool },
}

/// The sidebar's tabs, in tab-bar order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SidebarTab {
    /// The note tree.
    #[default]
    Notes,
    Search,
    Todos,
    Tags,
}

impl SidebarTab {
    pub const ALL: [SidebarTab; 4] = [
        SidebarTab::Notes,
        SidebarTab::Search,
        SidebarTab::Todos,
        SidebarTab::Tags,
    ];

    /// The tab's position in the tab bar.
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|tab| *tab == self).unwrap_or(0)
    }

    pub fn label(self) -> &'static str {
        match self {
            SidebarTab::Notes => "Notes",
            SidebarTab::Search => "Search",
            SidebarTab::Todos => "Todos",
            SidebarTab::Tags => "Tags",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            SidebarTab::Notes => Icon::new(AppIcon::FolderTree),
            SidebarTab::Search => Icon::new(IconName::Search),
            SidebarTab::Todos => Icon::new(AppIcon::ListTodo),
            SidebarTab::Tags => Icon::new(AppIcon::Tag),
        }
    }
}

/// The height of every list row: GPUI Kit's list needs rows of one height.
pub(crate) const ROW_HEIGHT: Pixels = px(26.);

/// What an empty list shows instead of rows.
pub(crate) fn empty_message(text: impl Into<SharedString>, cx: &App) -> impl IntoElement {
    div()
        .p_3()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(text.into())
}

/// The error line under a panel, when its last operation failed.
pub(crate) fn error_line(error: Option<SharedString>, cx: &App) -> Option<impl IntoElement> {
    error.map(|error| {
        div()
            .flex_none()
            .px_3()
            .py_2()
            .text_sm()
            .text_color(cx.theme().danger)
            .child(error)
    })
}

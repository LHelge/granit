//! The tags panel: every tag of the cave with its number of notes. A click
//! on a tag lists its notes below it (one tag at a time, like the Tauri
//! app's accordion); a click on a note opens it. The tags are the core's
//! [`granit_core::cave::Cave::list_tags`], rerun after every cave change
//! while the panel is shown.

use gpui::{
    div, App, AppContext as _, Context, Entity, EventEmitter, IntoElement, ParentElement, Render,
    SharedString, Styled, Subscription, Task, Window,
};
use gpui_kit::component::{
    h_flex,
    list::{List, ListDelegate, ListEvent, ListItem, ListState},
    ActiveTheme, Icon, IconName, IndexPath, Sizable,
};
use granit_core::cave::CaveError;
use granit_types::TagMap;
use log::warn;

use super::{empty_message, error_line, PanelEvent, ROW_HEIGHT};
use crate::assets::{note_icon_path, AppIcon};
use crate::events::{CaveEvent, CaveEvents};
use crate::host::Host;

/// One row of the tags list.
#[derive(Clone, Debug, PartialEq, Eq)]
enum TagRow {
    Tag {
        name: String,
        count: usize,
        expanded: bool,
    },
    /// A note of the expanded tag.
    Note { slug: String, icon: Option<String> },
}

/// The rows of `tags`, with the notes of `expanded` under it.
fn tag_rows(tags: &TagMap, expanded: Option<&str>) -> Vec<TagRow> {
    let mut rows = Vec::new();
    for (name, notes) in &tags.tags {
        let is_expanded = expanded == Some(name.as_str());
        rows.push(TagRow::Tag {
            name: name.clone(),
            count: notes.len(),
            expanded: is_expanded,
        });
        if is_expanded {
            rows.extend(notes.iter().map(|note| TagRow::Note {
                slug: note.slug.clone(),
                icon: note.icon.clone(),
            }));
        }
    }
    rows
}

/// The list delegate.
pub struct TagRows {
    rows: Vec<TagRow>,
}

impl ListDelegate for TagRows {
    type Item = ListItem;

    fn items_count(&self, _: usize, _: &App) -> usize {
        self.rows.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<ListItem> {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let item = ListItem::new(ix.row).h(ROW_HEIGHT).px_2().py_0().text_sm();
        Some(match self.rows.get(ix.row)? {
            TagRow::Tag {
                name,
                count,
                expanded,
            } => item.child(
                h_flex()
                    .gap_1()
                    .child(
                        Icon::new(if *expanded {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .xsmall()
                        .text_color(muted),
                    )
                    .child(Icon::new(AppIcon::Tag).xsmall())
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child(name.clone()),
                    )
                    .child(div().text_xs().text_color(muted).child(count.to_string())),
            ),
            TagRow::Note { slug, icon } => item.child(
                h_flex()
                    .gap_1()
                    .pl_6()
                    .child(
                        Icon::default()
                            .path(note_icon_path(icon.as_deref()))
                            .xsmall(),
                    )
                    .child(div().flex_1().min_w_0().truncate().child(slug.clone())),
            ),
        })
    }

    fn render_empty(
        &mut self,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> impl IntoElement {
        empty_message("No tags found", cx)
    }

    fn set_selected_index(
        &mut self,
        _: Option<IndexPath>,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) {
    }
}

pub struct TagsPanel {
    list: Entity<ListState<TagRows>>,
    tags: TagMap,
    /// The tag whose notes are listed.
    expanded: Option<String>,
    /// Whether the panel is the sidebar's shown tab.
    visible: bool,
    /// A cave change happened while hidden: rerun the query once shown.
    stale: bool,
    error: Option<SharedString>,
    _refresh: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<PanelEvent> for TagsPanel {}

impl TagsPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let list = cx.new(|cx| ListState::new(TagRows { rows: Vec::new() }, window, cx));
        let subscriptions = vec![
            cx.subscribe(&list, |this, _, event: &ListEvent, cx| {
                if let ListEvent::Confirm(ix) = event {
                    this.activate(ix.row, cx);
                }
            }),
            cx.subscribe(&CaveEvents::entity(cx), |this, _, event, cx| {
                if *event == CaveEvent::Opened {
                    this.expanded = None;
                    this.error = None;
                }
                if this.visible {
                    this.refresh(cx);
                } else {
                    this.stale = true;
                }
            }),
        ];
        Self {
            list,
            tags: TagMap::default(),
            expanded: None,
            visible: false,
            stale: true,
            error: None,
            _refresh: None,
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

    /// Rerun the tag query on the background executor.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.stale = false;
        let core = Host::core(cx);
        let query = cx.background_spawn(async move { core.with_cave(|cave| cave.list_tags()) });
        self._refresh = Some(cx.spawn(async move |this, cx| {
            let result = query.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(tags) => {
                        this.tags = tags;
                        this.error = None;
                    }
                    Err(CaveError::NoCaveOpen) => this.tags = TagMap::default(),
                    Err(err) => {
                        warn!("Could not list the tags: {err}");
                        this.error = Some(format!("Could not list the tags: {err}").into());
                    }
                }
                // A tag no note carries any more folds away.
                if let Some(tag) = &this.expanded {
                    if !this.tags.tags.contains_key(tag) {
                        this.expanded = None;
                    }
                }
                this.rebuild_rows(cx);
            })
            .ok();
        }));
    }

    fn rebuild_rows(&mut self, cx: &mut Context<Self>) {
        let rows = tag_rows(&self.tags, self.expanded.as_deref());
        self.list.update(cx, |list, cx| {
            list.delegate_mut().rows = rows;
            cx.notify();
        });
        cx.notify();
    }

    /// A row was clicked (or Enter pressed on it).
    fn activate(&mut self, row: usize, cx: &mut Context<Self>) {
        let Some(row) = self.list.read(cx).delegate().rows.get(row).cloned() else {
            return;
        };
        match row {
            TagRow::Tag { name, expanded, .. } => {
                self.expanded = (!expanded).then_some(name);
                self.rebuild_rows(cx);
            }
            TagRow::Note { slug, .. } => cx.emit(PanelEvent::OpenNote { slug, reveal: None }),
        }
    }

    /// Click row `row`, for tests.
    #[cfg(test)]
    pub fn click_row(&mut self, row: usize, cx: &mut Context<Self>) {
        self.activate(row, cx);
    }

    /// The rows as text, for tests: `tag (n)` and, indented, the notes of
    /// the expanded tag.
    #[cfg(test)]
    pub fn visible_rows(&self, cx: &App) -> Vec<String> {
        self.list
            .read(cx)
            .delegate()
            .rows
            .iter()
            .map(|row| match row {
                TagRow::Tag { name, count, .. } => format!("{name} ({count})"),
                TagRow::Note { slug, .. } => format!("  {slug}"),
            })
            .collect()
    }
}

impl Render for TagsPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(div().flex_1().min_h_0().px_1().child(List::new(&self.list)))
            .children(error_line(self.error.clone(), cx))
    }
}

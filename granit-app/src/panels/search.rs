//! The search panel: full-text search over note bodies as the user types.
//!
//! The query box is GPUI Kit's searchable list input; each change reruns
//! [`granit_core::cave::Cave::search_lines`] (case-insensitive substring,
//! after a short [`SEARCH_DELAY`] so a burst of keystrokes searches once) on
//! the background executor. Results list each matching note with its
//! matching lines under it; a click on a line opens the note at that line,
//! on a note at its first match. A cave change reruns the query while the
//! panel is shown.

use std::time::Duration;

use gpui::{
    div, App, AppContext as _, Context, Entity, EventEmitter, IntoElement, ParentElement, Render,
    SharedString, Styled, Subscription, Task, Window,
};
use gpui_kit::component::{
    h_flex,
    list::{List, ListDelegate, ListEvent, ListItem, ListState},
    ActiveTheme, Icon, IconName, IndexPath, Sizable,
};
use granit_core::cave::{CaveError, NoteMatches};
use log::warn;

use super::{empty_message, PanelEvent, ROW_HEIGHT};
use crate::events::CaveEvents;
use crate::host::Host;
use crate::note_view::Reveal;

/// How long typing must pause before the query runs.
pub const SEARCH_DELAY: Duration = Duration::from_millis(150);

/// At most this many notes are listed.
const MAX_NOTES: usize = 100;

/// One row of the results.
#[derive(Clone, Debug, PartialEq, Eq)]
enum SearchRow {
    /// A matching note, with its first matching body line.
    Note {
        slug: String,
        matches: usize,
        first_line: usize,
    },
    /// A matching body line (0-based) of the note above.
    Line {
        slug: String,
        line: usize,
        text: String,
    },
}

fn search_rows(results: Vec<NoteMatches>) -> Vec<SearchRow> {
    let mut rows = Vec::new();
    for note in results {
        rows.push(SearchRow::Note {
            slug: note.slug.clone(),
            matches: note.lines.len(),
            first_line: note.lines.first().map_or(0, |line| line.line),
        });
        rows.extend(note.lines.into_iter().map(|line| SearchRow::Line {
            slug: note.slug.clone(),
            line: line.line,
            text: line.text.trim().to_string(),
        }));
    }
    rows
}

/// The list delegate: the query and its results.
pub struct SearchRows {
    query: String,
    rows: Vec<SearchRow>,
    error: Option<SharedString>,
}

impl SearchRows {
    /// Run `self.query` after [`SEARCH_DELAY`] and show the results. The
    /// returned task is the search: dropping it (a newer query) cancels it.
    /// Two searches can be in flight (one typed, one rerun after a cave
    /// change, owned by different tasks), so a result for a query that is
    /// no longer the box's is dropped.
    fn search(&mut self, cx: &mut Context<ListState<Self>>) -> Task<()> {
        let query = self.query.clone();
        if query.is_empty() {
            self.rows.clear();
            self.error = None;
            cx.notify();
            return Task::ready(());
        }
        let core = Host::core(cx);
        cx.spawn(async move |list, cx| {
            cx.background_executor().timer(SEARCH_DELAY).await;
            let searched = query.clone();
            let result = cx
                .background_spawn(async move {
                    core.with_cave(|cave| cave.search_lines(&searched, Some(MAX_NOTES)))
                })
                .await;
            list.update(cx, |list, cx| {
                let rows = list.delegate_mut();
                if rows.query != query {
                    return;
                }
                match result {
                    Ok(results) => {
                        rows.rows = search_rows(results);
                        rows.error = None;
                    }
                    Err(CaveError::NoCaveOpen) => rows.rows.clear(),
                    Err(err) => {
                        warn!("Search failed: {err}");
                        rows.rows.clear();
                        rows.error = Some(format!("Search failed: {err}").into());
                    }
                }
                cx.notify();
            })
            .ok();
        })
    }
}

impl ListDelegate for SearchRows {
    type Item = ListItem;

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        self.query = query.to_string();
        self.search(cx)
    }

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
            SearchRow::Note { slug, matches, .. } => item.child(
                h_flex()
                    .gap_1()
                    .child(Icon::new(IconName::FileText).xsmall().text_color(muted))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child(slug.clone()),
                    )
                    .child(div().text_xs().text_color(muted).child(matches.to_string())),
            ),
            SearchRow::Line { text, .. } => item.child(
                div()
                    .pl_5()
                    .text_xs()
                    .text_color(muted)
                    .truncate()
                    .child(text.clone()),
            ),
        })
    }

    fn render_empty(
        &mut self,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> impl IntoElement {
        let text = match (&self.error, self.query.is_empty()) {
            (Some(error), _) => error.clone(),
            (None, true) => "Type to search the notes".into(),
            (None, false) => "No results".into(),
        };
        empty_message(text, cx)
    }

    fn set_selected_index(
        &mut self,
        _: Option<IndexPath>,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) {
    }
}

pub struct SearchPanel {
    list: Entity<ListState<SearchRows>>,
    /// Whether the panel is the sidebar's shown tab.
    visible: bool,
    /// A cave change happened while hidden: rerun the query once shown.
    stale: bool,
    _refresh: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<PanelEvent> for SearchPanel {}

impl SearchPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let rows = SearchRows {
            query: String::new(),
            rows: Vec::new(),
            error: None,
        };
        let list = cx.new(|cx| ListState::new(rows, window, cx).searchable(true));
        let subscriptions = vec![
            cx.subscribe(&list, |this, _, event: &ListEvent, cx| {
                if let ListEvent::Confirm(ix) = event {
                    this.activate(ix.row, cx);
                }
            }),
            cx.subscribe(&CaveEvents::entity(cx), |this, _, _, cx| {
                if this.visible {
                    this.refresh(cx);
                } else {
                    this.stale = true;
                }
            }),
        ];
        Self {
            list,
            visible: false,
            stale: false,
            _refresh: None,
            _subscriptions: subscriptions,
        }
    }

    /// The panel was shown or hidden; a stale panel searches again when
    /// shown.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        self.visible = visible;
        if visible && self.stale {
            self.refresh(cx);
        }
    }

    /// Put the keyboard focus in the query box.
    pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.list.update(cx, |list, cx| list.focus(window, cx));
    }

    /// Run the current query again (the cave changed).
    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.stale = false;
        let task = self
            .list
            .update(cx, |list, cx| list.delegate_mut().search(cx));
        self._refresh = Some(task);
    }

    /// A row was clicked (or Enter pressed on it).
    fn activate(&mut self, row: usize, cx: &mut Context<Self>) {
        let Some(row) = self.list.read(cx).delegate().rows.get(row).cloned() else {
            return;
        };
        let (slug, line) = match row {
            SearchRow::Note {
                slug, first_line, ..
            } => (slug, first_line),
            SearchRow::Line { slug, line, .. } => (slug, line),
        };
        cx.emit(PanelEvent::OpenNote {
            slug,
            reveal: Some(Reveal::Line(line)),
        });
    }

    /// Type `query` into the query box, for tests.
    #[cfg(test)]
    pub fn set_query(&mut self, query: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.list
            .update(cx, |list, cx| list.set_query(query, window, cx));
    }

    /// Click row `row`, for tests.
    #[cfg(test)]
    pub fn click_row(&mut self, row: usize, cx: &mut Context<Self>) {
        self.activate(row, cx);
    }

    /// The rows as text, for tests: `note (n)` and, indented, the lines.
    #[cfg(test)]
    pub fn visible_rows(&self, cx: &App) -> Vec<String> {
        self.list
            .read(cx)
            .delegate()
            .rows
            .iter()
            .map(|row| match row {
                SearchRow::Note { slug, matches, .. } => format!("{slug} ({matches})"),
                SearchRow::Line { text, .. } => format!("  {text}"),
            })
            .collect()
    }
}

impl Render for SearchPanel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .px_1()
            .child(List::new(&self.list).search_placeholder("Search notes…"))
    }
}

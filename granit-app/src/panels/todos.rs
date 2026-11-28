//! The todos panel: the open tasks of every note, grouped by note, with the
//! done ones folded away under a "Completed" row (as in the Tauri app).
//!
//! A click on a note row opens the note, on a todo opens it at the todo's
//! line, and on a checkbox asks the workspace to toggle it
//! ([`PanelEvent::ToggleTodo`]). The list is the core's
//! [`granit_core::cave::Cave::list_todos`], rerun after every cave change
//! while the panel is shown.

use gpui::{
    div, prelude::FluentBuilder as _, App, AppContext as _, Context, Entity, EventEmitter,
    IntoElement, ParentElement, Render, SharedString, Styled, Subscription, Task, WeakEntity,
    Window,
};
use gpui_kit::component::{
    checkbox::Checkbox,
    h_flex,
    list::{List, ListDelegate, ListEvent, ListItem, ListState},
    ActiveTheme, Icon, IconName, IndexPath, Sizable,
};
use granit_core::cave::CaveError;
use granit_types::{TodoItem, TodoList};
use log::warn;

use super::{empty_message, error_line, PanelEvent, ROW_HEIGHT};
use crate::events::{CaveEvent, CaveEvents};
use crate::host::Host;
use crate::note_view::Reveal;

/// One row of the todos list.
#[derive(Clone, Debug)]
enum TodoRow {
    /// The note the todos below it are in.
    Note(String),
    /// A todo; `checked` for a done one.
    Todo { item: TodoItem, checked: bool },
    /// The fold above the done todos.
    Completed { count: usize, shown: bool },
}

/// The rows of a todo list: open todos grouped by note, then the fold and,
/// when `show_completed`, the done ones grouped the same way.
fn todo_rows(todos: &TodoList, show_completed: bool) -> Vec<TodoRow> {
    fn grouped(rows: &mut Vec<TodoRow>, items: &[TodoItem], checked: bool) {
        let mut note: Option<&str> = None;
        for item in items {
            if note != Some(item.slug.as_str()) {
                note = Some(item.slug.as_str());
                rows.push(TodoRow::Note(item.slug.clone()));
            }
            rows.push(TodoRow::Todo {
                item: item.clone(),
                checked,
            });
        }
    }
    let mut rows = Vec::new();
    grouped(&mut rows, &todos.incomplete, false);
    if !todos.completed.is_empty() {
        rows.push(TodoRow::Completed {
            count: todos.completed.len(),
            shown: show_completed,
        });
        if show_completed {
            grouped(&mut rows, &todos.completed, true);
        }
    }
    rows
}

/// The list delegate: the rows, and the panel a checkbox reports to.
pub struct TodoRows {
    rows: Vec<TodoRow>,
    panel: WeakEntity<TodosPanel>,
}

impl ListDelegate for TodoRows {
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
        let row = self.rows.get(ix.row)?;
        let item = ListItem::new(ix.row).h(ROW_HEIGHT).px_2().py_0().text_sm();
        Some(match row {
            TodoRow::Note(slug) => item.child(
                h_flex()
                    .gap_1()
                    .text_xs()
                    .text_color(muted)
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(Icon::new(IconName::FileText).xsmall())
                    .child(div().min_w_0().truncate().child(slug.clone())),
            ),
            TodoRow::Todo {
                item: todo,
                checked,
            } => {
                let panel = self.panel.clone();
                let toggled = todo.clone();
                let checked = *checked;
                item.child(
                    h_flex()
                        .gap_2()
                        .child(
                            Checkbox::new(("todo-check", ix.row))
                                .small()
                                .checked(checked)
                                .tab_stop(false)
                                .on_click(move |_, _, cx| {
                                    // The row itself would open the note.
                                    cx.stop_propagation();
                                    let item = toggled.clone();
                                    panel
                                        .update(cx, |_, cx| {
                                            cx.emit(PanelEvent::ToggleTodo { item, checked })
                                        })
                                        .ok();
                                }),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .when(checked, |text| text.line_through().text_color(muted))
                                .child(todo.text.clone()),
                        ),
                )
            }
            TodoRow::Completed { count, shown } => item.child(
                h_flex()
                    .gap_1()
                    .text_xs()
                    .text_color(muted)
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(
                        Icon::new(if *shown {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .xsmall(),
                    )
                    .child(format!("Completed ({count})")),
            ),
        })
    }

    fn render_empty(
        &mut self,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> impl IntoElement {
        empty_message("No todos found", cx)
    }

    fn set_selected_index(
        &mut self,
        _: Option<IndexPath>,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) {
    }
}

pub struct TodosPanel {
    list: Entity<ListState<TodoRows>>,
    todos: TodoList,
    show_completed: bool,
    /// Whether the panel is the sidebar's shown tab.
    visible: bool,
    /// A cave change happened while hidden: rerun the query once shown.
    stale: bool,
    error: Option<SharedString>,
    _refresh: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<PanelEvent> for TodosPanel {}

impl TodosPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let rows = TodoRows {
            rows: Vec::new(),
            panel: cx.entity().downgrade(),
        };
        let list = cx.new(|cx| ListState::new(rows, window, cx));
        let subscriptions = vec![
            cx.subscribe(&list, |this, _, event: &ListEvent, cx| {
                if let ListEvent::Confirm(ix) = event {
                    this.activate(ix.row, cx);
                }
            }),
            cx.subscribe(&CaveEvents::entity(cx), |this, _, event, cx| {
                if *event == CaveEvent::Opened {
                    this.show_completed = false;
                    this.error = None;
                }
                this.cave_changed(cx);
            }),
        ];
        Self {
            list,
            todos: TodoList::default(),
            show_completed: false,
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

    /// Show why toggling a todo failed (the workspace does the toggling).
    pub fn show_error(&mut self, error: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.error = Some(error.into());
        cx.notify();
    }

    fn cave_changed(&mut self, cx: &mut Context<Self>) {
        if self.visible {
            self.refresh(cx);
        } else {
            self.stale = true;
        }
    }

    /// Rerun the todo query on the background executor.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.stale = false;
        let core = Host::core(cx);
        let query = cx.background_spawn(async move { core.with_cave(|cave| cave.list_todos()) });
        self._refresh = Some(cx.spawn(async move |this, cx| {
            let result = query.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(todos) => {
                        this.todos = todos;
                        this.error = None;
                    }
                    Err(CaveError::NoCaveOpen) => this.todos = TodoList::default(),
                    Err(err) => {
                        warn!("Could not list the todos: {err}");
                        this.error = Some(format!("Could not list the todos: {err}").into());
                    }
                }
                this.rebuild_rows(cx);
            })
            .ok();
        }));
    }

    fn rebuild_rows(&mut self, cx: &mut Context<Self>) {
        let rows = todo_rows(&self.todos, self.show_completed);
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
            TodoRow::Note(slug) => cx.emit(PanelEvent::OpenNote { slug, reveal: None }),
            TodoRow::Todo { item, .. } => {
                let line =
                    Host::core(cx).with_cave(|cave| cave.todo_body_line(&item.slug, item.line));
                cx.emit(PanelEvent::OpenNote {
                    slug: item.slug,
                    // A note that cannot be read still opens (and shows why).
                    reveal: line.ok().map(Reveal::Line),
                });
            }
            TodoRow::Completed { .. } => {
                self.show_completed = !self.show_completed;
                self.rebuild_rows(cx);
            }
        }
    }

    /// Click row `row`, for tests.
    #[cfg(test)]
    pub fn click_row(&mut self, row: usize, cx: &mut Context<Self>) {
        self.activate(row, cx);
    }

    /// Click the checkbox of row `row`, for tests.
    #[cfg(test)]
    pub fn click_checkbox(&mut self, row: usize, cx: &mut Context<Self>) {
        if let Some(TodoRow::Todo { item, checked }) =
            self.list.read(cx).delegate().rows.get(row).cloned()
        {
            cx.emit(PanelEvent::ToggleTodo { item, checked });
        }
    }

    /// The rows as text, for tests: `note`, `[ ] todo` / `[x] todo`, and
    /// `Completed (n)`.
    #[cfg(test)]
    pub fn visible_rows(&self, cx: &App) -> Vec<String> {
        self.list
            .read(cx)
            .delegate()
            .rows
            .iter()
            .map(|row| match row {
                TodoRow::Note(slug) => slug.clone(),
                TodoRow::Todo { item, checked } => {
                    format!("[{}] {}", if *checked { "x" } else { " " }, item.text)
                }
                TodoRow::Completed { count, .. } => format!("Completed ({count})"),
            })
            .collect()
    }

    #[cfg(test)]
    pub fn error(&self) -> Option<&SharedString> {
        self.error.as_ref()
    }
}

impl Render for TodosPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(div().flex_1().min_h_0().px_1().child(List::new(&self.list)))
            .children(error_line(self.error.clone(), cx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(slug: &str, line: usize, text: &str) -> TodoItem {
        TodoItem {
            slug: slug.into(),
            relative_path: format!("{slug}.md"),
            line,
            text: text.into(),
        }
    }

    #[test]
    fn rows_group_todos_by_note_and_fold_the_done_ones() {
        let todos = TodoList {
            incomplete: vec![
                item("a", 1, "one"),
                item("a", 2, "two"),
                item("b", 1, "three"),
            ],
            completed: vec![item("b", 2, "done")],
        };
        let labels = |rows: Vec<TodoRow>| -> Vec<String> {
            rows.into_iter()
                .map(|row| match row {
                    TodoRow::Note(slug) => slug,
                    TodoRow::Todo { item, .. } => item.text,
                    TodoRow::Completed { count, shown } => format!("completed {count} {shown}"),
                })
                .collect()
        };
        assert_eq!(
            labels(todo_rows(&todos, false)),
            ["a", "one", "two", "b", "three", "completed 1 false"]
        );
        assert_eq!(
            labels(todo_rows(&todos, true)),
            [
                "a",
                "one",
                "two",
                "b",
                "three",
                "completed 1 true",
                "b",
                "done"
            ]
        );
        assert!(todo_rows(&TodoList::default(), true).is_empty());
    }
}

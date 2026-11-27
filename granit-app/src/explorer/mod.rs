//! The sidebar explorer: the cave's folders and notes as a tree.
//!
//! The tree mirrors the core's note and folder lists and rebuilds itself on
//! every [`CaveEvent`], keeping expanded folders expanded and the open note
//! selected. Selecting a note emits [`ExplorerEvent::OpenNote`]; the
//! workspace opens it and reports the open note back with
//! [`Explorer::set_active`]. Folders and notes are created, renamed
//! (inline), moved (drag and drop) and deleted through [`crate::cave_ops`].

mod row;
pub mod tree_model;

use std::collections::{HashMap, HashSet};

use gpui::{
    div, App, AppContext, Context, Entity, EventEmitter, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, Styled, Subscription, Window,
};
use gpui_kit::component::{
    button::{Button, ButtonVariants},
    h_flex,
    input::{InputEvent, InputState},
    menu::{PopupMenu, PopupMenuItem},
    tree::{tree, TreeEntry, TreeEvent, TreeState},
    ActiveTheme, Sizable,
};
use granit_core::cave::{CaveError, DocumentMeta};
use log::warn;

use crate::assets::AppIcon;
use crate::cave_ops;
use crate::events::{CaveEvent, CaveEvents};
use tree_model::{ancestor_folders, build_tree, item_kind, note_item_id, to_tree_items, ItemKind};

/// What the explorer asks of the workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorerEvent {
    /// The user selected this note.
    OpenNote(String),
}

/// A tree entry that can be renamed, moved or deleted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// A note, by slug.
    Note(String),
    /// A folder, by its cave-relative path.
    Folder(String),
}

impl Entry {
    fn from_item_id(id: &str) -> Option<Self> {
        match item_kind(id)? {
            ItemKind::Note(slug) => Some(Entry::Note(slug.to_string())),
            ItemKind::Folder(path) => Some(Entry::Folder(path.to_string())),
        }
    }

    /// The name shown in the tree (and edited by a rename).
    fn name(&self) -> &str {
        match self {
            Entry::Note(slug) => slug,
            Entry::Folder(path) => path.rsplit_once('/').map_or(path, |(_, name)| name),
        }
    }
}

/// The entry being renamed inline, with its text input.
struct Rename {
    entry: Entry,
    input: Entity<InputState>,
    _events: Subscription,
}

pub struct Explorer {
    tree: Entity<TreeState>,
    /// The cave's notes by slug, for the row icons and favorite stars.
    notes: HashMap<String, DocumentMeta>,
    /// Paths of the folders shown expanded.
    expanded: HashSet<String>,
    /// The note open in the main area.
    active: Option<String>,
    rename: Option<Rename>,
    /// The last failed operation, shown under the tree.
    error: Option<SharedString>,
    /// Whether a cave is open.
    cave_open: bool,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<ExplorerEvent> for Explorer {}

impl Explorer {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let tree = cx.new(|cx| TreeState::new(cx));
        let subscriptions = vec![
            cx.observe(&tree, |this, tree, cx| this.follow_selection(&tree, cx)),
            cx.subscribe(&tree, |this, _, event: &TreeEvent, _| match event {
                TreeEvent::Expanded(id) | TreeEvent::Collapsed(id) => {
                    if let Some(ItemKind::Folder(path)) = item_kind(id) {
                        if matches!(event, TreeEvent::Expanded(_)) {
                            this.expanded.insert(path.to_string());
                        } else {
                            this.expanded.remove(path);
                        }
                    }
                }
            }),
            cx.subscribe(&CaveEvents::entity(cx), |this, _, event, cx| {
                if *event == CaveEvent::Opened {
                    this.expanded.clear();
                    this.active = None;
                    this.rename = None;
                    this.error = None;
                }
                this.refresh(cx);
            }),
        ];
        let mut this = Self {
            tree,
            notes: HashMap::new(),
            expanded: HashSet::new(),
            active: None,
            rename: None,
            error: None,
            cave_open: false,
            _subscriptions: subscriptions,
        };
        this.refresh(cx);
        this
    }

    /// Rebuild the tree from the cave, keeping expansion and the selection
    /// of the open note.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let (notes, folders) = match cave_ops::list_tree(cx) {
            Ok(lists) => lists,
            Err(CaveError::NoCaveOpen) => (Vec::new(), Vec::new()),
            Err(err) => {
                self.error = Some(format!("Could not list notes: {err}").into());
                cx.notify();
                return;
            }
        };
        self.cave_open = crate::host::Host::cave_path(cx).is_some();
        // Folders that no longer exist stop being expanded.
        self.expanded.retain(|path| folders.contains(path));
        self.notes = notes
            .iter()
            .map(|meta| (meta.slug.clone(), meta.clone()))
            .collect();
        let items = to_tree_items(&build_tree(notes, folders), &self.expanded);
        self.tree.update(cx, |tree, cx| tree.set_items(items, cx));
        self.select_active(cx);
        cx.notify();
    }

    /// The note open in the main area changed: select it, expanding the
    /// folders above it.
    pub fn set_active(&mut self, slug: Option<String>, cx: &mut Context<Self>) {
        self.active = slug;
        self.select_active(cx);
        cx.notify();
    }

    /// The selected tree entry.
    #[cfg(test)]
    pub fn selected(&self, cx: &App) -> Option<Entry> {
        let tree = self.tree.read(cx);
        Entry::from_item_id(&tree.selected_item()?.id)
    }

    /// The entry being renamed, and the text typed so far.
    #[cfg(test)]
    pub fn renaming(&self, cx: &App) -> Option<(Entry, SharedString)> {
        let rename = self.rename.as_ref()?;
        Some((rename.entry.clone(), rename.input.read(cx).value()))
    }

    /// The last error, if the last operation failed.
    #[cfg(test)]
    pub fn error(&self) -> Option<&SharedString> {
        self.error.as_ref()
    }

    /// The labels of the visible rows, indented two spaces per level, for
    /// tests.
    #[cfg(test)]
    pub fn visible_rows(&self, cx: &App) -> Vec<String> {
        let tree = self.tree.read(cx);
        (0..)
            .map_while(|ix| tree.entry(ix))
            .map(|entry| format!("{}{}", "  ".repeat(entry.depth()), entry.item().label))
            .collect()
    }

    /// The icon and favorite star each visible note row shows, by slug, for
    /// tests.
    #[cfg(test)]
    pub fn note_decorations(&self, cx: &App) -> Vec<(String, row::NoteDecoration)> {
        let tree = self.tree.read(cx);
        (0..)
            .map_while(|ix| tree.entry(ix))
            .filter_map(|entry| match item_kind(&entry.item().id)? {
                ItemKind::Note(slug) => {
                    Some((slug.to_string(), row::note_decoration(self.notes.get(slug))))
                }
                ItemKind::Folder(_) => None,
            })
            .collect()
    }

    /// Select the visible row `ix`, as a click or the arrow keys do, for
    /// tests.
    #[cfg(test)]
    pub fn select_row(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.tree
            .update(cx, |tree, cx| tree.set_selected_index(Some(ix), cx));
    }

    fn select_active(&mut self, cx: &mut Context<Self>) {
        let Some(slug) = self.active.clone() else {
            self.tree
                .update(cx, |tree, cx| tree.set_selected_index(None, cx));
            return;
        };
        if let Some(meta) = self.notes.get(&slug) {
            self.expanded.extend(ancestor_folders(&meta.relative_path));
        }
        let id = SharedString::from(note_item_id(&slug));
        self.tree.update(cx, |tree, cx| {
            tree.reveal_item(&id, gpui::ScrollStrategy::Center, cx);
            let ix = tree.index_of(&id);
            tree.set_selected_index(ix, cx);
        });
    }

    /// A note selected in the tree (click or arrow keys) opens it.
    fn follow_selection(&mut self, tree: &Entity<TreeState>, cx: &mut Context<Self>) {
        let Some(Entry::Note(slug)) = tree
            .read(cx)
            .selected_item()
            .and_then(|item| Entry::from_item_id(&item.id))
        else {
            return;
        };
        if self.active.as_deref() != Some(slug.as_str()) {
            cx.emit(ExplorerEvent::OpenNote(slug));
        }
    }

    /// Show the result of an operation: clear the error line on success,
    /// show it on failure.
    fn report<T>(&mut self, action: &str, result: Result<T, CaveError>) -> Option<T> {
        match result {
            Ok(value) => {
                self.error = None;
                Some(value)
            }
            Err(err) => {
                warn!("{action} failed: {err}");
                self.error = Some(format!("{action} failed: {err}").into());
                None
            }
        }
    }

    /// Create a note in `folder` (`None`: the cave root), open it and start
    /// renaming it.
    pub fn new_note(
        &mut self,
        folder: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let result = cave_ops::create_note(folder.as_deref(), cx);
        if let Some(meta) = self.report("Creating the note", result) {
            self.expanded.extend(ancestor_folders(&meta.relative_path));
            cx.emit(ExplorerEvent::OpenNote(meta.slug.clone()));
            self.start_rename(Entry::Note(meta.slug), window, cx);
        }
        cx.notify();
    }

    /// Create a folder in `parent` (`None`: the cave root) and start renaming
    /// it.
    pub fn new_folder(
        &mut self,
        parent: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let result = cave_ops::create_folder(parent.as_deref(), cx);
        if let Some(path) = self.report("Creating the folder", result) {
            self.expanded.extend(parent);
            self.refresh(cx);
            self.start_rename(Entry::Folder(path), window, cx);
        }
        cx.notify();
    }

    /// Replace `entry`'s row with a text input holding its name. Enter or
    /// leaving the input renames it, Escape cancels.
    pub fn start_rename(&mut self, entry: Entry, window: &mut Window, cx: &mut Context<Self>) {
        let name = entry.name().to_string();
        let input = cx.new(|cx| InputState::new(window, cx).default_value(name));
        let events = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                this.commit_rename(cx);
            }
        });
        input.update(cx, |input, cx| {
            input.focus(window, cx);
            input.select_all(window, cx);
        });
        self.rename = Some(Rename {
            entry,
            input,
            _events: events,
        });
        cx.notify();
    }

    /// Stop renaming without changing anything.
    pub fn cancel_rename(&mut self, cx: &mut Context<Self>) {
        self.rename = None;
        cx.notify();
    }

    /// Rename the entry to the typed name; an empty or unchanged name
    /// changes nothing.
    pub fn commit_rename(&mut self, cx: &mut Context<Self>) {
        let Some(rename) = self.rename.take() else {
            return;
        };
        let new_name = rename.input.read(cx).value().trim().to_string();
        cx.notify();
        if new_name.is_empty() || new_name == rename.entry.name() {
            return;
        }
        match rename.entry {
            Entry::Note(slug) => {
                let result = cave_ops::rename_note(&slug, &new_name, cx);
                self.report("Renaming the note", result);
            }
            Entry::Folder(path) => {
                let result = cave_ops::rename_folder(&path, &new_name, cx);
                if let Some(new_path) = self.report("Renaming the folder", result) {
                    self.follow_folder(&path, &new_path, cx);
                }
            }
        }
    }

    /// Delete `entry` (a folder with everything in it).
    pub fn delete(&mut self, entry: Entry, cx: &mut Context<Self>) {
        match entry {
            Entry::Note(slug) => {
                let result = cave_ops::delete_note(&slug, cx);
                self.report("Deleting the note", result);
            }
            Entry::Folder(path) => {
                let result = cave_ops::delete_folder(&path, cx);
                self.report("Deleting the folder", result);
            }
        }
        cx.notify();
    }

    /// Move `entry` into `folder` (`None`: the cave root).
    pub fn move_entry(&mut self, entry: Entry, folder: Option<String>, cx: &mut Context<Self>) {
        match entry {
            Entry::Note(slug) => {
                let result = cave_ops::move_note(&slug, folder.as_deref(), cx);
                if self.report("Moving the note", result).is_some() {
                    self.expanded.extend(folder.clone());
                    self.refresh(cx);
                }
            }
            Entry::Folder(path) => {
                let result = cave_ops::move_folder(&path, folder.as_deref(), cx);
                if let Some(new_path) = self.report("Moving the folder", result) {
                    self.expanded.extend(folder.clone());
                    self.follow_folder(&path, &new_path, cx);
                }
            }
        }
        cx.notify();
    }

    /// Keep a renamed or moved folder (and its subfolders) expanded.
    fn follow_folder(&mut self, old: &str, new: &str, cx: &mut Context<Self>) {
        let old_prefix = format!("{old}/");
        let moved: Vec<String> = self
            .expanded
            .iter()
            .filter(|path| *path == old || path.starts_with(&old_prefix))
            .cloned()
            .collect();
        for path in moved {
            self.expanded.remove(&path);
            self.expanded.insert(format!("{new}{}", &path[old.len()..]));
        }
        self.refresh(cx);
    }

    /// The context menu of a tree entry.
    fn context_menu(this: gpui::WeakEntity<Self>, entry: &TreeEntry, menu: PopupMenu) -> PopupMenu {
        let Some(target) = Entry::from_item_id(&entry.item().id) else {
            return menu;
        };
        let action = |f: fn(&mut Self, Entry, &mut Window, &mut Context<Self>)| {
            let this = this.clone();
            let target = target.clone();
            move |_: &gpui::ClickEvent, window: &mut Window, cx: &mut App| {
                this.update(cx, |this, cx| f(this, target.clone(), window, cx))
                    .ok();
            }
        };
        let rename = PopupMenuItem::new("Rename").on_click(action(|this, entry, window, cx| {
            this.start_rename(entry, window, cx)
        }));
        match &target {
            Entry::Note(_) => menu.item(rename).item(
                PopupMenuItem::new("Delete note")
                    .on_click(action(|this, entry, _, cx| this.delete(entry, cx))),
            ),
            Entry::Folder(_) => menu
                .item(PopupMenuItem::new("New note here").on_click(action(
                    |this, entry, window, cx| {
                        if let Entry::Folder(path) = entry {
                            this.new_note(Some(path), window, cx);
                        }
                    },
                )))
                .item(PopupMenuItem::new("New folder here").on_click(action(
                    |this, entry, window, cx| {
                        if let Entry::Folder(path) = entry {
                            this.new_folder(Some(path), window, cx);
                        }
                    },
                )))
                .item(rename)
                .item(
                    PopupMenuItem::new("Delete folder")
                        .on_click(action(|this, entry, _, cx| this.delete(entry, cx))),
                ),
        }
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        h_flex()
            .flex_none()
            .px_3()
            .py_1()
            .gap_1()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("Notes"),
            )
            .child(
                Button::new("new-note")
                    .ghost()
                    .xsmall()
                    .icon(AppIcon::FilePlus)
                    .tooltip("New note")
                    .on_click(cx.listener(|this, _, window, cx| this.new_note(None, window, cx))),
            )
            .child(
                Button::new("new-folder")
                    .ghost()
                    .xsmall()
                    .icon(AppIcon::FolderPlus)
                    .tooltip("New folder")
                    .on_click(cx.listener(|this, _, window, cx| this.new_folder(None, window, cx))),
            )
    }
}

impl Render for Explorer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.cave_open {
            return div().into_any_element();
        }
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let danger = theme.danger;
        let this = cx.entity().downgrade();
        let render_row = row::renderer(this.clone());
        let tree = tree(&self.tree, render_row).context_menu(move |_, entry, menu, _, _| {
            Self::context_menu(this.clone(), entry, menu)
        });
        let body = if self.notes.is_empty() && self.tree.read(cx).entry(0).is_none() {
            div()
                .p_3()
                .text_sm()
                .text_color(muted)
                .child("No notes yet")
                .into_any_element()
        } else {
            tree.into_any_element()
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(self.render_header(cx))
            .child(
                div()
                    .id("explorer-tree")
                    .flex_1()
                    .min_h_0()
                    .px_1()
                    // Dropping on the empty space below the tree moves to
                    // the cave root; rows handle drops onto folders.
                    .on_drop(cx.listener(|this, dragged: &row::DraggedEntry, _, cx| {
                        this.move_entry(dragged.entry.clone(), None, cx);
                    }))
                    .child(body),
            )
            .children(self.error.clone().map(|error| {
                div()
                    .flex_none()
                    .px_3()
                    .py_2()
                    .text_sm()
                    .text_color(danger)
                    .child(error)
            }))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests;

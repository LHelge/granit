//! Rendering of one explorer row: chevron, icon, favorite star and name,
//! or the rename input; plus dragging the row and dropping onto it.

use gpui::{
    div, prelude::FluentBuilder as _, px, App, AppContext as _, Context, InteractiveElement,
    IntoElement, KeyDownEvent, ParentElement, Render, SharedString,
    StatefulInteractiveElement as _, Styled, WeakEntity, Window,
};
use gpui_kit::component::{
    h_flex, input::Input, list::ListItem, tree::TreeEntry, ActiveTheme, Icon, IconName, Sizable,
};

use granit_core::cave::DocumentMeta;

use super::{tree_model::ItemKind, ClickKind, Entry, Explorer};
use crate::assets::note_icon_path;
use crate::cave_ops::parent_of;

const INDENT: f32 = 14.;
const ICON_WIDTH: f32 = 14.;

/// The payload of a row being dragged: its entry, or every note of the
/// multi-selection it belongs to.
#[derive(Clone, Debug)]
pub struct DraggedEntry {
    pub entries: Vec<Entry>,
    label: SharedString,
}

/// What follows the pointer while a row is dragged.
struct DragPreview(SharedString);

impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .px_2()
            .py_1()
            .rounded_md()
            .text_sm()
            .bg(theme.popover)
            .text_color(theme.popover_foreground)
            .border_1()
            .border_color(theme.border)
            .child(self.0.clone())
    }
}

/// What a note row shows besides its name: the icon from its frontmatter
/// and whether it gets the favorite star.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteDecoration {
    pub icon: SharedString,
    pub favorite: bool,
}

/// The decoration of the note `meta` (unknown notes get the default icon).
pub fn note_decoration(meta: Option<&DocumentMeta>) -> NoteDecoration {
    NoteDecoration {
        icon: note_icon_path(meta.and_then(|meta| meta.icon.as_deref())),
        favorite: meta.is_some_and(|meta| meta.favorite.unwrap_or(false)),
    }
}

/// The row renderer for the explorer's tree.
pub fn renderer(
    explorer: WeakEntity<Explorer>,
) -> impl Fn(usize, &TreeEntry, bool, &mut Window, &mut App) -> ListItem + 'static {
    move |ix, entry, _selected, _window, cx| render_row(&explorer, ix, entry, cx)
}

fn render_row(
    explorer: &WeakEntity<Explorer>,
    ix: usize,
    entry: &TreeEntry,
    cx: &mut App,
) -> ListItem {
    let item = entry.item();
    let target = Entry::from_item_id(&item.id);
    let (meta, rename_input, multi_selected, dragged) = match (explorer.upgrade(), &target) {
        (Some(explorer), Some(target)) => {
            let explorer = explorer.read(cx);
            let (meta, multi_selected) = match target {
                Entry::Note(slug) => (
                    explorer.notes.get(slug).cloned(),
                    explorer.in_multi_selection(slug),
                ),
                Entry::Folder(_) => (None, false),
            };
            let input = explorer
                .rename
                .as_ref()
                .filter(|rename| rename.entry == *target)
                .map(|rename| rename.input.clone());
            (
                meta,
                input,
                multi_selected,
                explorer.dragged_entries(target),
            )
        }
        _ => (None, None, false, Vec::new()),
    };
    let theme = cx.theme();
    let is_folder = matches!(super::item_kind(&item.id), Some(ItemKind::Folder(_)));

    let chevron = if entry.is_folder() {
        let name = if entry.is_expanded() {
            IconName::ChevronDown
        } else {
            IconName::ChevronRight
        };
        Icon::new(name)
            .xsmall()
            .text_color(theme.muted_foreground)
            .into_any_element()
    } else {
        div().w(px(ICON_WIDTH)).flex_none().into_any_element()
    };

    let decoration = note_decoration(meta.as_ref());
    let icon = if is_folder {
        let name = if entry.is_expanded() {
            IconName::FolderOpen
        } else {
            IconName::Folder
        };
        Icon::new(name).small().text_color(theme.muted_foreground)
    } else {
        Icon::default()
            .path(decoration.icon)
            .small()
            .text_color(theme.primary)
    };
    let star = (!is_folder && decoration.favorite)
        .then(|| Icon::new(IconName::Star).xsmall().text_color(theme.warning));

    let renaming = rename_input.is_some();
    let label = match rename_input {
        Some(input) => div()
            .flex_1()
            .min_w_0()
            .child(Input::new(&input).xsmall())
            .into_any_element(),
        None => div()
            .min_w_0()
            .truncate()
            .child(item.label.clone())
            .into_any_element(),
    };

    let row = ListItem::new(ix)
        .pl(px(6. + INDENT * entry.depth() as f32))
        // The notes of a multi-selection besides the open one (which the
        // tree highlights as its selected row).
        .when(multi_selected, |row| row.bg(theme.selection.opacity(0.35)))
        .child(
            h_flex()
                .gap_1()
                .min_w_0()
                .w_full()
                .child(chevron)
                .child(icon)
                .children(star)
                .child(label),
        );

    let Some(target) = target else {
        return row;
    };
    // A click on a note updates the selection first (the tree's own click
    // handler, which selects the row and so opens the note, runs after
    // it); a Ctrl/Cmd-click stops there, so the note does not open.
    let row = match &target {
        Entry::Note(slug) if !renaming => {
            let explorer = explorer.clone();
            let slug = slug.clone();
            let selector = format!("explorer-note-{slug}");
            row.debug_selector(move || selector)
                .on_click(move |event, _, cx| {
                    let kind = ClickKind::from_modifiers(event.modifiers());
                    let opens = explorer
                        .update(cx, |explorer, cx| explorer.click_note(&slug, kind, cx))
                        .unwrap_or(true);
                    if !opens {
                        cx.stop_propagation();
                    }
                })
        }
        _ => row,
    };
    // Escape in the rename input cancels the rename.
    let row = {
        let explorer = explorer.clone();
        row.capture_key_down(move |event: &KeyDownEvent, _, cx| {
            if event.keystroke.key == "escape" {
                explorer
                    .update(cx, |explorer, cx| {
                        if explorer.rename.is_some() {
                            explorer.cancel_rename(cx);
                        }
                    })
                    .ok();
            }
        })
    };
    // Dropping onto a folder moves into it; onto a note, next to it.
    let drop_folder = match &target {
        Entry::Folder(path) => Some(path.clone()),
        Entry::Note(_) => meta
            .as_ref()
            .and_then(|meta| parent_of(&meta.relative_path).map(str::to_string)),
    };
    let label = match dragged.len() {
        0 | 1 => item.label.clone(),
        count => format!("{count} notes").into(),
    };
    let dragged = DraggedEntry {
        entries: if dragged.is_empty() {
            vec![target]
        } else {
            dragged
        },
        label,
    };
    let drop_target = explorer.clone();
    let hover = theme.accent;
    row.on_drag(dragged, |dragged, _, _, cx| {
        cx.new(|_| DragPreview(dragged.label.clone()))
    })
    .drag_over::<DraggedEntry>(move |style, _, _, _| style.bg(hover))
    .on_drop(move |dragged: &DraggedEntry, _, cx| {
        drop_target
            .update(cx, |explorer, cx| {
                explorer.move_entries(dragged.entries.clone(), drop_folder.clone(), cx)
            })
            .ok();
    })
}

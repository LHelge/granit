//! Sidebar row rendering for the note tree.

use gpui::{div, px, App, IntoElement, ParentElement, Styled, Window};
use gpui_kit::component::{
    h_flex, list::ListItem, tree::TreeEntry, ActiveTheme, Icon, IconName, Sizable,
};

use crate::tree_model::{item_kind, ItemKind};

const INDENT: f32 = 14.;
const ICON_WIDTH: f32 = 14.;

/// Render one visible tree entry: chevron, icon and label, indented by depth.
pub fn render_entry(
    ix: usize,
    entry: &TreeEntry,
    _selected: bool,
    _: &mut Window,
    cx: &mut App,
) -> ListItem {
    let theme = cx.theme();
    let item = entry.item();
    let is_folder = matches!(item_kind(&item.id), Some(ItemKind::Folder(_)));

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
        div().w(px(ICON_WIDTH)).into_any_element()
    };

    let icon = match (is_folder, entry.is_expanded()) {
        (true, true) => IconName::FolderOpen,
        (true, false) => IconName::Folder,
        (false, _) => IconName::FileText,
    };

    ListItem::new(ix)
        .pl(px(6. + INDENT * entry.depth() as f32))
        .child(
            h_flex()
                .gap_1()
                .min_w_0()
                .child(chevron)
                .child(Icon::new(icon).small().text_color(theme.muted_foreground))
                .child(div().min_w_0().truncate().child(item.label.clone())),
        )
}

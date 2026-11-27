//! The app's asset source: GPUI Kit's bundled assets plus the note icons.
//!
//! Note icons come from the curated catalog in `granit_types::NOTE_ICONS`
//! (Lucide and Material Design SVG data). They are served as SVG files at
//! `note-icons/<Id>.svg`, built from the catalog's path data on request, so
//! GPUI caches each rendered icon by its path like any other asset. Draw one
//! with `Icon::default().path(note_icon_path(meta.icon.as_deref()))`.
//!
//! The few app icons GPUI Kit's default bundle lacks are served the same way
//! from `app-icons/<name>.svg` ([`AppIcon`]).

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};
use gpui_kit::component::IconNamed;
use granit_types::NOTE_ICONS;
use icondata_core::IconData;

const NOTE_ICON_DIR: &str = "note-icons/";
const APP_ICON_DIR: &str = "app-icons/";

/// App icons beyond GPUI Kit's default bundle; use them like Kit icons,
/// `Icon::new(AppIcon::FilePlus)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppIcon {
    FilePlus,
    FolderPlus,
    PanelRight,
    Send,
    SquarePen,
    Wrench,
    CircleAlert,
    Settings,
    Palette,
    NotebookPen,
    Bot,
    Trash,
    FolderTree,
    ListTodo,
    Tag,
}

impl AppIcon {
    const ALL: [AppIcon; 15] = [
        AppIcon::FilePlus,
        AppIcon::FolderPlus,
        AppIcon::PanelRight,
        AppIcon::Send,
        AppIcon::SquarePen,
        AppIcon::Wrench,
        AppIcon::CircleAlert,
        AppIcon::Settings,
        AppIcon::Palette,
        AppIcon::NotebookPen,
        AppIcon::Bot,
        AppIcon::Trash,
        AppIcon::FolderTree,
        AppIcon::ListTodo,
        AppIcon::Tag,
    ];

    fn name(self) -> &'static str {
        match self {
            AppIcon::FilePlus => "file-plus",
            AppIcon::FolderPlus => "folder-plus",
            AppIcon::PanelRight => "panel-right",
            AppIcon::Send => "send",
            AppIcon::SquarePen => "square-pen",
            AppIcon::Wrench => "wrench",
            AppIcon::CircleAlert => "circle-alert",
            AppIcon::Settings => "settings",
            AppIcon::Palette => "palette",
            AppIcon::NotebookPen => "notebook-pen",
            AppIcon::Bot => "bot",
            AppIcon::Trash => "trash-2",
            AppIcon::FolderTree => "folder-tree",
            AppIcon::ListTodo => "list-todo",
            AppIcon::Tag => "tag",
        }
    }

    fn data(self) -> &'static IconData {
        match self {
            AppIcon::FilePlus => icondata_lu::LuFilePlus,
            AppIcon::FolderPlus => icondata_lu::LuFolderPlus,
            AppIcon::PanelRight => icondata_lu::LuPanelRight,
            AppIcon::Send => icondata_lu::LuSend,
            AppIcon::SquarePen => icondata_lu::LuSquarePen,
            AppIcon::Wrench => icondata_lu::LuWrench,
            AppIcon::CircleAlert => icondata_lu::LuCircleAlert,
            AppIcon::Settings => icondata_lu::LuSettings,
            AppIcon::Palette => icondata_lu::LuPalette,
            AppIcon::NotebookPen => icondata_lu::LuNotebookPen,
            AppIcon::Bot => icondata_lu::LuBot,
            AppIcon::Trash => icondata_lu::LuTrash2,
            AppIcon::FolderTree => icondata_lu::LuFolderTree,
            AppIcon::ListTodo => icondata_lu::LuListTodo,
            AppIcon::Tag => icondata_lu::LuTag,
        }
    }
}

impl IconNamed for AppIcon {
    fn path(self) -> SharedString {
        format!("{APP_ICON_DIR}{}.svg", self.name()).into()
    }
}

/// The icon id drawn for notes without an icon, or with an unknown one.
const DEFAULT_NOTE_ICON: &str = "FileText";

/// The asset path of a note's icon, from its frontmatter `icon` id. Unknown
/// and missing ids get the default note icon, as in the reader.
pub fn note_icon_path(icon: Option<&str>) -> SharedString {
    let id = icon
        .filter(|id| NOTE_ICONS.iter().any(|entry| entry.id == *id))
        .unwrap_or(DEFAULT_NOTE_ICON);
    format!("{NOTE_ICON_DIR}{id}.svg").into()
}

/// GPUI Kit's assets, with the note icons added.
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(file) = path.strip_prefix(NOTE_ICON_DIR) {
            let icon = file
                .strip_suffix(".svg")
                .and_then(|id| NOTE_ICONS.iter().find(|entry| entry.id == id));
            return Ok(icon.map(|entry| Cow::Owned(svg_document(entry.icon).into_bytes())));
        }
        if path.starts_with(APP_ICON_DIR) {
            let icon = AppIcon::ALL.into_iter().find(|icon| icon.path() == path);
            return Ok(icon.map(|icon| Cow::Owned(svg_document(icon.data()).into_bytes())));
        }
        gpui_kit::assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = gpui_kit::assets::Assets.list(path)?;
        let ours = NOTE_ICONS
            .iter()
            .map(|entry| SharedString::from(format!("{NOTE_ICON_DIR}{}.svg", entry.id)))
            .chain(AppIcon::ALL.into_iter().map(IconNamed::path));
        paths.extend(ours.filter(|icon| icon.starts_with(path)));
        Ok(paths)
    }
}

/// A standalone SVG document for an icon of the catalog.
fn svg_document(icon: &IconData) -> String {
    let attributes = [
        ("viewBox", icon.view_box),
        ("fill", icon.fill),
        ("stroke", icon.stroke),
        ("stroke-width", icon.stroke_width),
        ("stroke-linecap", icon.stroke_linecap),
        ("stroke-linejoin", icon.stroke_linejoin),
        ("style", icon.style),
    ];
    let mut svg = String::from(r#"<svg xmlns="http://www.w3.org/2000/svg""#);
    for (name, value) in attributes {
        if let Some(value) = value {
            svg.push_str(&format!(r#" {name}="{value}""#));
        }
    }
    svg.push('>');
    svg.push_str(icon.data);
    svg.push_str("</svg>");
    svg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_icons_resolve_and_others_fall_back_to_the_note_icon() {
        assert_eq!(note_icon_path(Some("Star")).as_ref(), "note-icons/Star.svg");
        assert_eq!(
            note_icon_path(Some("NoSuchIcon")).as_ref(),
            "note-icons/FileText.svg"
        );
        assert_eq!(note_icon_path(None).as_ref(), "note-icons/FileText.svg");
    }

    #[test]
    fn every_catalog_icon_is_served_as_an_svg() {
        for entry in NOTE_ICONS {
            let path = format!("note-icons/{}.svg", entry.id);
            let svg = Assets.load(&path).unwrap().expect(&path);
            let svg = std::str::from_utf8(&svg).unwrap();
            assert!(svg.starts_with("<svg xmlns="), "{path}: {svg}");
            assert!(svg.contains("viewBox="), "{path}: {svg}");
            assert!(svg.ends_with("</svg>"), "{path}");
        }
    }

    #[test]
    fn app_icons_are_served() {
        for icon in AppIcon::ALL {
            let path = icon.path();
            let svg = Assets.load(&path).unwrap().expect(&path);
            assert!(svg.starts_with(b"<svg xmlns="), "{path}");
        }
        assert!(Assets.load("app-icons/nope.svg").unwrap().is_none());
    }

    #[test]
    fn unknown_note_icons_are_missing_and_kit_assets_still_load() {
        assert!(Assets.load("note-icons/NoSuchIcon.svg").unwrap().is_none());
        assert!(Assets.load("icons/folder.svg").unwrap().is_some());
    }

    #[test]
    fn listing_includes_our_icons() {
        let listed = Assets.list("app-icons/").unwrap();
        assert_eq!(listed.len(), AppIcon::ALL.len());
        assert!(Assets
            .list("note-icons/")
            .unwrap()
            .iter()
            .any(|path| path.as_ref() == "note-icons/Star.svg"));
    }
}

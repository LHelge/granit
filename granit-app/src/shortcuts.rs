//! The "Keyboard shortcuts" dialog, as in the Tauri app's keybinds modal,
//! opened from the sidebar's keyboard button.
//!
//! The list is read from GPUI's keymap when the dialog opens, so a binding
//! added to `workspace::bind_keys` or `note_view::bind_keys` shows up
//! without touching this module:
//! - every binding of an action in the app's own namespaces ([`GROUPS`]) is
//!   listed under that namespace's group, described by the action's doc
//!   comment (its first line; the action's name when it has none);
//! - zorite-editor's actions carry no docs, so the formatting ones worth
//!   listing are named in [`EDITOR_ACTIONS`], with their keys still read
//!   from the keymap;
//! - completion triggers are typed text, not bindings ([`TYPED`]).
//!
//! Keys are drawn per OS with GPUI Kit's `Kbd::format` (`⌘B` on macOS,
//! `Ctrl+B` elsewhere). Of an action bound on several keys the first one
//! for this platform is shown: zorite-editor binds `cmd-` and `ctrl-` on
//! every OS, so `cmd-` (drawn as `Win` off macOS) is dropped off macOS,
//! and on macOS `cmd-` wins over `ctrl-`.

use gpui::{
    div, prelude::FluentBuilder as _, px, App, AsKeystroke as _, InteractiveElement as _,
    IntoElement, Keystroke, ParentElement, StatefulInteractiveElement as _, Styled, Window,
};
use gpui_kit::component::{h_flex, kbd::Kbd, v_flex, ActiveTheme, WindowExt as _};

/// The action namespaces whose bindings are listed, and their group titles,
/// in the order shown.
pub const GROUPS: &[(&str, &str)] = &[("granit", "General"), ("note_view", "Notes")];

/// zorite-editor's actions listed under "Formatting", with what they do.
pub const EDITOR_ACTIONS: &[(&str, &str)] = &[
    ("zorite_editor::Bold", "Bold"),
    ("zorite_editor::Italic", "Italic"),
    ("zorite_editor::Underline", "Underline"),
    ("zorite_editor::Strike", "Strikethrough"),
    ("zorite_editor::Highlight", "Highlight"),
    ("zorite_editor::Code", "Inline code"),
    ("zorite_editor::Indent", "Indent"),
    ("zorite_editor::Outdent", "Outdent"),
    ("zorite_editor::MoveLinesUp", "Move lines up"),
    ("zorite_editor::MoveLinesDown", "Move lines down"),
    ("zorite_editor::DuplicateLines", "Duplicate lines"),
    ("zorite_editor::Undo", "Undo"),
    ("zorite_editor::Redo", "Redo"),
];

/// Typed triggers, listed under "Completion".
const TYPED: &[(&str, &str)] = &[("[[", "Wiki-link completion"), ("> [!", "Alert completion")];

/// A group of shortcuts shown as one section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShortcutGroup {
    pub title: &'static str,
    pub shortcuts: Vec<Shortcut>,
}

/// One row: the keys as drawn on this OS and what they do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shortcut {
    pub keys: String,
    pub description: String,
}

/// The shortcuts of the app's keymap, grouped as the dialog shows them.
/// Empty groups are left out.
pub fn groups(cx: &App) -> Vec<ShortcutGroup> {
    let keymap = cx.key_bindings();
    let keymap = keymap.borrow();
    let docs = cx.action_documentation();
    // (action name, keystrokes) in keymap order.
    let bindings: Vec<(&'static str, Vec<Keystroke>)> = keymap
        .bindings()
        .map(|binding| {
            let keys = binding
                .keystrokes()
                .iter()
                .map(|key| key.as_keystroke().clone())
                .collect();
            (binding.action().name(), keys)
        })
        .collect();

    let mut groups: Vec<ShortcutGroup> = GROUPS
        .iter()
        .map(|(namespace, title)| {
            let mut names: Vec<&'static str> = Vec::new();
            for (name, _) in &bindings {
                if action_namespace(name) == Some(*namespace) && !names.contains(name) {
                    names.push(name);
                }
            }
            let shortcuts = names
                .into_iter()
                .filter_map(|name| {
                    let keys = keys_for(name, &bindings)?;
                    let description = docs
                        .get(name)
                        .and_then(|doc| describe(doc))
                        .unwrap_or_else(|| humanize(name));
                    Some(Shortcut { keys, description })
                })
                .collect();
            ShortcutGroup { title, shortcuts }
        })
        .collect();
    groups.push(ShortcutGroup {
        title: "Formatting",
        shortcuts: EDITOR_ACTIONS
            .iter()
            .filter_map(|(name, description)| {
                Some(Shortcut {
                    keys: keys_for(name, &bindings)?,
                    description: description.to_string(),
                })
            })
            .collect(),
    });
    groups.push(ShortcutGroup {
        title: "Completion",
        shortcuts: TYPED
            .iter()
            .map(|(keys, description)| Shortcut {
                keys: keys.to_string(),
                description: description.to_string(),
            })
            .collect(),
    });
    groups.retain(|group| !group.shortcuts.is_empty());
    groups
}

/// `granit::OpenCave` → `granit`.
fn action_namespace(name: &str) -> Option<&str> {
    name.rsplit_once("::").map(|(namespace, _)| namespace)
}

/// The keys `name` is bound to on this platform, as drawn: the first
/// binding that suits it ([`suits_platform`]), each keystroke of a chord
/// separated by a space.
fn keys_for(name: &str, bindings: &[(&'static str, Vec<Keystroke>)]) -> Option<String> {
    let candidates: Vec<&Vec<Keystroke>> = bindings
        .iter()
        .filter(|(bound, keys)| *bound == name && !keys.is_empty())
        .map(|(_, keys)| keys)
        .collect();
    let has_platform = candidates
        .iter()
        .any(|keys| keys.iter().any(|key| key.modifiers.platform));
    let chosen = candidates
        .into_iter()
        .find(|keys| suits_platform(keys, cfg!(target_os = "macos"), has_platform))?;
    Some(chosen.iter().map(Kbd::format).collect::<Vec<_>>().join(" "))
}

/// Whether a binding is the one to show on this platform. Off macOS, `cmd-`
/// is the Windows key, which the app never means. On macOS a `ctrl-`
/// binding is the other-OS twin of a `cmd-` one (zorite-editor binds both),
/// so it is skipped when the action has a `cmd-` binding.
fn suits_platform(keys: &[Keystroke], mac: bool, action_has_platform: bool) -> bool {
    let uses = |f: fn(&Keystroke) -> bool| keys.iter().any(f);
    if mac {
        !(action_has_platform
            && uses(|key| key.modifiers.control)
            && !uses(|key| key.modifiers.platform))
    } else {
        !uses(|key| key.modifiers.platform)
    }
}

/// The first line of an action's doc comment, without Markdown code marks
/// or a closing period.
fn describe(doc: &str) -> Option<String> {
    let paragraph: Vec<&str> = doc
        .lines()
        .map(str::trim)
        .skip_while(|line| line.is_empty())
        .take_while(|line| !line.is_empty())
        .collect();
    if paragraph.is_empty() {
        return None;
    }
    let text = paragraph.join(" ").replace('`', "");
    Some(text.trim_end_matches('.').to_string())
}

/// `note_view::ToggleTask` → `Toggle task`.
fn humanize(name: &str) -> String {
    let name = name.rsplit("::").next().unwrap_or(name);
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push(' ');
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// Open the shortcuts dialog in `window`.
pub fn open(window: &mut Window, cx: &mut App) {
    let groups = groups(cx);
    window.open_dialog(cx, move |dialog, _, cx| {
        dialog
            .title("Keyboard shortcuts")
            .w(px(440.))
            .child(render_groups(&groups, cx))
    });
}

fn render_groups(groups: &[ShortcutGroup], cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let (muted, border) = (theme.muted_foreground, theme.border);
    let radius = theme.radius;
    div()
        .id("shortcut-groups")
        .max_h(px(480.))
        .overflow_y_scroll()
        .child(v_flex().gap_4().children(groups.iter().map(|group| {
            v_flex()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(muted)
                        .child(group.title.to_uppercase()),
                )
                .child(
                    v_flex()
                        .border_1()
                        .border_color(border)
                        .rounded(radius)
                        .overflow_hidden()
                        .children(group.shortcuts.iter().enumerate().map(|(ix, shortcut)| {
                            h_flex()
                                .justify_between()
                                .gap_4()
                                .px_4()
                                .py_1p5()
                                .when(ix > 0, |row| row.border_t_1().border_color(border))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(muted)
                                        .child(shortcut.description.clone()),
                                )
                                .child(key_label(shortcut.keys.clone(), cx))
                        })),
                )
        })))
}

/// Keys drawn like GPUI Kit's `Kbd`, from already formatted text (typed
/// triggers such as `[[` are not keystrokes).
fn key_label(text: String, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex_none()
        .py_0p5()
        .px_1p5()
        .rounded(theme.radius)
        .bg(theme.muted)
        .text_color(theme.muted_foreground)
        .text_xs()
        .child(text)
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, Keystroke, TestAppContext};

    use super::*;
    use crate::host::HostOptions;
    use crate::tests::App;
    use crate::workspace::StartupCave;

    fn keys(source: &'static str) -> Vec<Keystroke> {
        source
            .split_whitespace()
            .map(|key| Keystroke::parse(key).unwrap())
            .collect()
    }

    #[test]
    fn descriptions_come_from_the_doc_comment_or_the_name() {
        assert_eq!(
            describe("Wrap the selection as a link, or insert `[[]]`.\n\nMore.").as_deref(),
            Some("Wrap the selection as a link, or insert [[]]")
        );
        assert_eq!(
            describe("\nMake the previous match current (opening\nthe bar).\n\nMore.").as_deref(),
            Some("Make the previous match current (opening the bar)")
        );
        assert_eq!(describe("  \n"), None);
        assert_eq!(humanize("note_view::ToggleTask"), "Toggle task");
        assert_eq!(humanize("Quit"), "Quit");
    }

    #[test]
    fn the_platform_binding_is_chosen() {
        // Off macOS `cmd-` is the Windows key.
        assert!(!suits_platform(&keys("cmd-b"), false, true));
        assert!(suits_platform(&keys("ctrl-b"), false, true));
        // On macOS `cmd-` wins over its `ctrl-` twin, but a `ctrl-` only
        // binding stays.
        assert!(suits_platform(&keys("cmd-b"), true, true));
        assert!(!suits_platform(&keys("ctrl-b"), true, true));
        assert!(suits_platform(&keys("ctrl-home"), true, false));
        assert!(suits_platform(&keys("tab"), true, true));
    }

    #[gpui::test]
    fn the_list_follows_the_keymap(cx: &mut TestAppContext) {
        let _app = App::start(HostOptions::default(), StartupCave::None, cx);
        let groups = cx.update(|cx| groups(cx));
        let titles: Vec<&str> = groups.iter().map(|group| group.title).collect();
        assert_eq!(titles, ["General", "Notes", "Formatting", "Completion"]);

        let find = |title: &str, description: &str| {
            groups
                .iter()
                .find(|group| group.title == title)
                .and_then(|group| {
                    group
                        .shortcuts
                        .iter()
                        .find(|shortcut| shortcut.description == description)
                })
                .map(|shortcut| shortcut.keys.clone())
        };
        // The keys come from the constants `bind_keys` registers, drawn as
        // this OS draws them.
        let drawn = |key: &'static str| Kbd::format(&Keystroke::parse(key).unwrap());
        assert_eq!(
            find("General", "Show or hide the agent chat"),
            Some(drawn(crate::workspace::TOGGLE_CHAT_KEY))
        );
        assert_eq!(
            find("Notes", "Write the open note now"),
            Some(drawn(crate::note_view::SAVE_KEY))
        );
        let bold = if cfg!(target_os = "macos") {
            "cmd-b"
        } else {
            "ctrl-b"
        };
        assert_eq!(find("Formatting", "Bold"), Some(drawn(bold)));
        assert_eq!(
            find("Completion", "Wiki-link completion").as_deref(),
            Some("[[")
        );
        assert_eq!(
            find("Completion", "Alert completion").as_deref(),
            Some("> [!")
        );

        // The editor extras: `-d` is select next (it shadows zorite's
        // Duplicate, which is not listed), duplicate lines stays on
        // `-shift-d`.
        let (d, shift_d) = if cfg!(target_os = "macos") {
            ("cmd-d", "cmd-shift-d")
        } else {
            ("ctrl-d", "ctrl-shift-d")
        };
        assert_eq!(
            find("Notes", "Select next occurrence"),
            Some(drawn(crate::note_view::SELECT_NEXT_KEY))
        );
        assert_eq!(drawn(crate::note_view::SELECT_NEXT_KEY), drawn(d));
        assert_eq!(find("Formatting", "Duplicate lines"), Some(drawn(shift_d)));
        assert_eq!(
            find("Notes", "Find next"),
            Some(drawn(crate::note_view::FIND_NEXT_KEY))
        );
        assert_eq!(
            find("Notes", "Find previous"),
            Some(drawn(crate::note_view::FIND_PREVIOUS_KEY))
        );
        let d_rows: Vec<&Shortcut> = groups
            .iter()
            .flat_map(|group| &group.shortcuts)
            .filter(|shortcut| shortcut.keys == drawn(d))
            .collect();
        assert_eq!(d_rows.len(), 1, "only select next is on -d: {d_rows:?}");

        // Every bound action of the app's namespaces is listed once.
        let general = &groups[0].shortcuts;
        assert_eq!(general.len(), 5, "{general:?}");

        // A binding added later shows up on the next open.
        cx.update(|cx| {
            cx.bind_keys([gpui::KeyBinding::new(
                crate::workspace::QUIT_KEY,
                crate::Quit,
                None,
            )])
        });
        let groups = cx.update(|cx| super::groups(cx));
        assert!(groups[0]
            .shortcuts
            .iter()
            .any(|shortcut| shortcut.description == "Quit Granit"));
        assert!(
            groups[0]
                .shortcuts
                .iter()
                .all(|shortcut| !shortcut.keys.contains("Win")),
            "no Windows-key bindings: {:?}",
            groups[0].shortcuts
        );
    }

    #[gpui::test]
    fn the_shortcuts_dialog_opens_and_draws(cx: &mut TestAppContext) {
        let app = App::start(HostOptions::default(), StartupCave::None, cx);
        cx.update_window(app.window, |_, window, cx| super::open(window, cx))
            .unwrap();
        cx.run_until_parked();
        app.draw(cx);
        let open = cx
            .update_window(app.window, |_, window, cx| window.has_active_dialog(cx))
            .unwrap();
        assert!(open, "the shortcuts dialog is shown");
    }
}

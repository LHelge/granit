//! Explorer operations against a real cave: create, rename, move and
//! delete notes and folders.

use gpui::{AppContext as _, Modifiers, TestAppContext, VisualTestContext};

use super::{ClickKind, Entry, Explorer};
use crate::tests::{fixture_cave, App};

/// The fixture cave plus the root notes `c` and `d`: the tree shows
/// `empty`, `sub` (collapsed), `a`, `c`, `d`.
fn selection_cave() -> tempfile::TempDir {
    let dir = fixture_cave();
    std::fs::write(dir.path().join("c.md"), "# C\n").unwrap();
    std::fs::write(dir.path().join("d.md"), "# D\n").unwrap();
    dir
}

/// Click the painted row of a note (`selector`: `explorer-note-<slug>`)
/// with `modifiers` held, as the mouse does.
fn click(app: &App, selector: &'static str, modifiers: Modifiers, cx: &mut TestAppContext) {
    app.draw(cx);
    let mut vcx = VisualTestContext::from_window(app.window, cx);
    let position = vcx
        .debug_bounds(selector)
        .unwrap_or_else(|| panic!("{selector} is painted"))
        .center();
    vcx.simulate_mouse_move(position, None, Modifiers::none());
    vcx.simulate_click(position, modifiers);
    cx.run_until_parked();
}

fn selected(app: &App, cx: &mut TestAppContext) -> Vec<String> {
    let explorer = app.explorer(cx);
    cx.read_entity(&explorer, |explorer: &Explorer, _| {
        explorer.selected_notes()
    })
}

fn ctrl() -> Modifiers {
    Modifiers {
        control: true,
        ..Modifiers::default()
    }
}

fn shift() -> Modifiers {
    Modifiers {
        shift: true,
        ..Modifiers::default()
    }
}

#[gpui::test]
fn modifier_clicks_build_a_selection_in_tree_order(cx: &mut TestAppContext) {
    let dir = selection_cave();
    let app = App::with_cave(dir.path(), cx);
    assert_eq!(app.rows(cx), vec!["empty", "sub", "a", "c", "d"]);

    click(&app, "explorer-note-a", Modifiers::none(), cx);
    assert_eq!(app.active_note(cx).as_deref(), Some("a"));
    assert_eq!(selected(&app, cx), ["a"]);

    // Ctrl/Cmd-click toggles without opening.
    click(&app, "explorer-note-c", ctrl(), cx);
    assert_eq!(app.active_note(cx).as_deref(), Some("a"), "c does not open");
    assert_eq!(selected(&app, cx), ["a", "c"]);
    let cmd = Modifiers {
        platform: true,
        ..Modifiers::default()
    };
    click(&app, "explorer-note-a", cmd, cx);
    assert_eq!(selected(&app, cx), ["c"]);
    assert_eq!(app.active_note(cx).as_deref(), Some("a"));
    click(&app, "explorer-note-a", ctrl(), cx);
    assert_eq!(selected(&app, cx), ["a", "c"]);

    // Shift-click selects the range from the last clicked note, in the
    // order the tree shows, and opens the clicked note.
    click(&app, "explorer-note-d", shift(), cx);
    assert_eq!(selected(&app, cx), ["a", "c", "d"]);
    assert_eq!(app.active_note(cx).as_deref(), Some("d"));
    // The range is redrawn from the same note.
    click(&app, "explorer-note-c", shift(), cx);
    assert_eq!(selected(&app, cx), ["a", "c"]);
    assert_eq!(app.active_note(cx).as_deref(), Some("c"));

    // A plain click selects just the clicked note.
    click(&app, "explorer-note-d", Modifiers::none(), cx);
    assert_eq!(selected(&app, cx), ["d"]);
    assert_eq!(app.active_note(cx).as_deref(), Some("d"));

    // A note opened elsewhere takes over a selection of one.
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_note(Some("a".into()), cx)
    });
    assert_eq!(selected(&app, cx), ["a"]);
}

#[gpui::test]
fn a_shift_click_ranges_over_open_folders_only(cx: &mut TestAppContext) {
    let dir = selection_cave();
    let app = App::with_cave(dir.path(), cx);
    app.update_explorer(cx, |explorer, _, cx| {
        explorer.click_note("a", ClickKind::Plain, cx);
        explorer.click_note("d", ClickKind::Extend, cx);
    });
    assert_eq!(selected(&app, cx), ["a", "c", "d"]);

    // With `sub` expanded, its `b` is in the range too.
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_note(Some("b".into()), cx)
    });
    assert_eq!(app.rows(cx), vec!["empty", "sub", "  b", "a", "c", "d"]);
    app.update_explorer(cx, |explorer, _, cx| {
        explorer.click_note("b", ClickKind::Plain, cx);
        explorer.click_note("c", ClickKind::Extend, cx);
    });
    assert_eq!(selected(&app, cx), ["b", "a", "c"]);
}

#[gpui::test]
fn dragging_a_selected_note_moves_the_whole_selection(cx: &mut TestAppContext) {
    let dir = selection_cave();
    let app = App::with_cave(dir.path(), cx);
    click(&app, "explorer-note-a", Modifiers::none(), cx);
    click(&app, "explorer-note-c", ctrl(), cx);

    let explorer = app.explorer(cx);
    let dragged = |slug: &str, cx: &mut TestAppContext| {
        let entry = Entry::Note(slug.into());
        cx.read_entity(&explorer, |explorer: &Explorer, _| {
            explorer.dragged_entries(&entry)
        })
    };
    assert_eq!(
        dragged("d", cx),
        [Entry::Note("d".into())],
        "an unselected note drags alone"
    );
    assert_eq!(
        dragged("c", cx),
        [Entry::Note("a".into()), Entry::Note("c".into())]
    );
    let folder = Entry::Folder("sub".into());
    let folder_dragged = cx.read_entity(&explorer, |explorer: &Explorer, _| {
        explorer.dragged_entries(&folder)
    });
    assert_eq!(folder_dragged, [folder]);

    // Drop them on the `sub` folder.
    let entries = dragged("c", cx);
    app.update_explorer(cx, |explorer, _, cx| {
        explorer.move_entries(entries, Some("sub".into()), cx)
    });
    assert!(dir.path().join("sub/a.md").exists());
    assert!(dir.path().join("sub/c.md").exists());
    assert_eq!(app.rows(cx), vec!["empty", "sub", "  a", "  b", "  c", "d"]);
    assert_eq!(
        app.active_note(cx).as_deref(),
        Some("a"),
        "the open note stays open"
    );
    assert_eq!(
        app.shown_body(cx).as_deref(),
        Some("# A\n\nAlpha links [[b]].\n")
    );
    assert_eq!(selected(&app, cx), ["a"], "the selection is spent");

    // A single note still drags and drops alone.
    app.update_explorer(cx, |explorer, _, cx| {
        explorer.move_entries(vec![Entry::Note("d".into())], Some("empty".into()), cx)
    });
    assert!(dir.path().join("empty/d.md").exists());
    app.draw(cx);
}

#[gpui::test]
fn deleting_a_selected_note_deletes_the_whole_selection(cx: &mut TestAppContext) {
    let dir = selection_cave();
    let app = App::with_cave(dir.path(), cx);
    click(&app, "explorer-note-a", Modifiers::none(), cx);
    click(&app, "explorer-note-c", ctrl(), cx);

    let explorer = app.explorer(cx);
    let targets = |slug: &str, cx: &mut TestAppContext| {
        cx.read_entity(&explorer, |explorer: &Explorer, _| explorer.notes_for(slug))
    };
    assert_eq!(targets("d", cx), ["d"], "a note outside the selection");
    let slugs = targets("c", cx);
    assert_eq!(slugs, ["a", "c"]);

    app.update_explorer(cx, |explorer, _, cx| explorer.delete_notes(slugs, cx));

    assert!(!dir.path().join("a.md").exists());
    assert!(!dir.path().join("c.md").exists());
    assert!(dir.path().join("d.md").exists());
    assert_eq!(app.rows(cx), vec!["empty", "sub", "d"]);
    assert_eq!(app.active_note(cx), None, "the deleted open note is closed");
    assert_eq!(selected(&app, cx), Vec::<String>::new());
    app.draw(cx);
}

/// Type `name` into the open rename input and press Enter.
fn type_and_commit(app: &App, name: &str, cx: &mut TestAppContext) {
    let name = name.to_string();
    app.update_explorer(cx, move |explorer, window, cx| {
        let input = explorer
            .rename
            .as_ref()
            .expect("a rename is open")
            .input
            .clone();
        input.update(cx, |input, cx| input.set_value(name, window, cx));
        explorer.commit_rename(cx);
    });
}

fn renaming(app: &App, cx: &mut TestAppContext) -> Option<(Entry, String)> {
    let explorer = app.explorer(cx);
    cx.read_entity(&explorer, |explorer: &Explorer, cx| {
        explorer
            .renaming(cx)
            .map(|(entry, text)| (entry, text.to_string()))
    })
}

fn error(app: &App, cx: &mut TestAppContext) -> Option<String> {
    let explorer = app.explorer(cx);
    cx.read_entity(&explorer, |explorer: &Explorer, _| {
        explorer.error().map(|error| error.to_string())
    })
}

#[gpui::test]
fn a_new_note_opens_and_is_named_inline(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);

    app.update_explorer(cx, |explorer, window, cx| {
        explorer.new_note(None, window, cx)
    });

    assert!(dir.path().join("untitled.md").exists());
    assert_eq!(app.active_note(cx).as_deref(), Some("untitled"));
    assert_eq!(
        renaming(&app, cx),
        Some((Entry::Note("untitled".into()), "untitled".into()))
    );
    app.draw(cx);

    type_and_commit(&app, "fresh", cx);

    assert!(dir.path().join("fresh.md").exists());
    assert!(!dir.path().join("untitled.md").exists());
    assert_eq!(renaming(&app, cx), None);
    assert_eq!(
        app.active_note(cx).as_deref(),
        Some("fresh"),
        "the open note follows"
    );
    assert_eq!(app.core_active_note(cx).as_deref(), Some("fresh"));
    assert_eq!(app.rows(cx), vec!["empty", "sub", "a", "fresh"]);
}

#[gpui::test]
fn a_new_note_in_a_folder_lands_there(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);

    app.update_explorer(cx, |explorer, window, cx| {
        explorer.new_note(Some("sub".into()), window, cx)
    });

    assert!(dir.path().join("sub/untitled.md").exists());
    assert_eq!(app.rows(cx), vec!["empty", "sub", "  b", "  untitled", "a"]);
}

#[gpui::test]
fn a_new_note_from_a_template_is_rendered_through_the_core(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    std::fs::create_dir_all(dir.path().join(".granit/templates")).unwrap();
    std::fs::write(
        dir.path().join(".granit/templates/meeting.md"),
        "---\nicon: Star\ntags: [meeting]\n---\n# {{ slug }}\n\n## Notes\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join(".granit/templates/day.md"),
        "# {{ date }}\n",
    )
    .unwrap();
    let app = App::with_cave(dir.path(), cx);
    cx.update(|cx| {
        crate::config_ops::update_config(cx, |config| {
            config.daily_note_template_slug = Some("day".into())
        })
    })
    .unwrap();

    // The daily-note template is not offered for other notes.
    let offered = cx.update(|cx| crate::cave_ops::templates_for_new_notes(cx).unwrap());
    let offered: Vec<&str> = offered.iter().map(|meta| meta.slug.as_str()).collect();
    assert_eq!(offered, ["meeting"]);

    app.update_explorer(cx, |explorer, window, cx| {
        explorer.new_note_from(Some("sub".into()), Some("meeting".into()), window, cx)
    });

    let raw = std::fs::read_to_string(dir.path().join("sub/untitled.md")).unwrap();
    assert!(raw.contains("icon: Star"), "{raw}");
    assert!(raw.contains("meeting"), "the tags are copied: {raw}");
    assert_eq!(app.active_note(cx).as_deref(), Some("untitled"));
    assert_eq!(
        app.shown_body(cx).as_deref(),
        Some("# untitled\n\n## Notes\n")
    );
    assert_eq!(
        renaming(&app, cx),
        Some((Entry::Note("untitled".into()), "untitled".into()))
    );
    assert_eq!(app.rows(cx), vec!["empty", "sub", "  b", "  untitled", "a"]);
    app.draw(cx);

    // A missing template is reported and creates nothing.
    app.update_explorer(cx, |explorer, window, cx| {
        explorer.new_note_from(None, Some("gone".into()), window, cx)
    });
    assert!(error(&app, cx).is_some());
    assert!(!dir.path().join("untitled.md").exists());
}

#[gpui::test]
fn renaming_a_note_rewrites_links_and_follows_the_open_note(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_note(Some("b".into()), cx)
    });

    app.update_explorer(cx, |explorer, window, cx| {
        explorer.start_rename(Entry::Note("b".into()), window, cx)
    });
    type_and_commit(&app, "bee", cx);

    assert!(dir.path().join("sub/bee.md").exists());
    let a = std::fs::read_to_string(dir.path().join("a.md")).unwrap();
    assert!(a.contains("[[bee]]"), "{a}");
    assert_eq!(app.active_note(cx).as_deref(), Some("bee"));
    assert_eq!(app.rows(cx), vec!["empty", "sub", "  bee", "a"]);
}

#[gpui::test]
fn an_unchanged_or_cancelled_rename_changes_nothing(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);

    app.update_explorer(cx, |explorer, window, cx| {
        explorer.start_rename(Entry::Note("a".into()), window, cx)
    });
    type_and_commit(&app, "  a  ", cx);
    assert!(dir.path().join("a.md").exists());

    app.update_explorer(cx, |explorer, window, cx| {
        explorer.start_rename(Entry::Note("a".into()), window, cx)
    });
    app.update_explorer(cx, |explorer, _, cx| explorer.cancel_rename(cx));
    assert_eq!(renaming(&app, cx), None);
    assert!(dir.path().join("a.md").exists());
    assert_eq!(error(&app, cx), None);
}

#[gpui::test]
fn a_failed_rename_is_shown(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);

    app.update_explorer(cx, |explorer, window, cx| {
        explorer.start_rename(Entry::Note("a".into()), window, cx)
    });
    type_and_commit(&app, "b", cx);

    assert!(dir.path().join("a.md").exists());
    let error = error(&app, cx).expect("the duplicate name is reported");
    assert!(error.starts_with("Renaming the note failed"), "{error}");
    app.draw(cx);
}

#[gpui::test]
fn deleting_the_open_note_closes_it(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_note(Some("a".into()), cx)
    });

    app.update_explorer(cx, |explorer, _, cx| {
        explorer.delete(Entry::Note("a".into()), cx)
    });

    assert!(!dir.path().join("a.md").exists());
    assert_eq!(app.active_note(cx), None);
    assert_eq!(app.core_active_note(cx), None);
    assert_eq!(app.rows(cx), vec!["empty", "sub"]);
}

#[gpui::test]
fn folders_are_created_renamed_and_deleted(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);

    app.update_explorer(cx, |explorer, window, cx| {
        explorer.new_folder(None, window, cx)
    });
    assert!(dir.path().join("new-folder").is_dir());
    assert_eq!(
        renaming(&app, cx),
        Some((Entry::Folder("new-folder".into()), "new-folder".into()))
    );
    type_and_commit(&app, "projects", cx);
    assert!(dir.path().join("projects").is_dir());
    assert_eq!(app.rows(cx), vec!["empty", "projects", "sub", "a"]);

    // A subfolder, then a rename of its parent keeps it expanded.
    app.update_explorer(cx, |explorer, window, cx| {
        explorer.new_folder(Some("projects".into()), window, cx)
    });
    type_and_commit(&app, "2026", cx);
    assert!(dir.path().join("projects/2026").is_dir());
    app.update_explorer(cx, |explorer, window, cx| {
        explorer.start_rename(Entry::Folder("projects".into()), window, cx)
    });
    type_and_commit(&app, "work", cx);
    assert!(dir.path().join("work/2026").is_dir());
    assert_eq!(app.rows(cx), vec!["empty", "sub", "work", "  2026", "a"]);

    app.update_explorer(cx, |explorer, _, cx| {
        explorer.delete(Entry::Folder("work".into()), cx)
    });
    assert!(!dir.path().join("work").exists());
    assert_eq!(app.rows(cx), vec!["empty", "sub", "a"]);
}

#[gpui::test]
fn deleting_the_folder_of_the_open_note_closes_it(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_note(Some("b".into()), cx)
    });

    app.update_explorer(cx, |explorer, _, cx| {
        explorer.delete(Entry::Folder("sub".into()), cx)
    });

    assert!(!dir.path().join("sub").exists());
    assert_eq!(app.active_note(cx), None);
    assert_eq!(app.rows(cx), vec!["empty", "a"]);
}

#[gpui::test]
fn notes_and_folders_move_by_drop(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_note(Some("a".into()), cx)
    });

    app.update_explorer(cx, |explorer, _, cx| {
        explorer.move_entry(Entry::Note("a".into()), Some("sub".into()), cx)
    });
    assert!(dir.path().join("sub/a.md").exists());
    assert_eq!(app.rows(cx), vec!["empty", "sub", "  a", "  b"]);
    assert_eq!(
        app.active_note(cx).as_deref(),
        Some("a"),
        "the open note stays open"
    );

    app.update_explorer(cx, |explorer, _, cx| {
        explorer.move_entry(Entry::Folder("sub".into()), Some("empty".into()), cx)
    });
    assert!(dir.path().join("empty/sub/b.md").exists());
    assert_eq!(app.rows(cx), vec!["empty", "  sub", "    a", "    b"]);

    // Back to the root.
    app.update_explorer(cx, |explorer, _, cx| {
        explorer.move_entry(Entry::Note("b".into()), None, cx)
    });
    assert!(dir.path().join("b.md").exists());
    assert_eq!(app.rows(cx), vec!["empty", "  sub", "    a", "b"]);

    // A folder cannot move into itself.
    app.update_explorer(cx, |explorer, _, cx| {
        explorer.move_entry(Entry::Folder("empty".into()), Some("empty/sub".into()), cx)
    });
    assert!(error(&app, cx).is_some());
    app.draw(cx);
}

/// Right-click the empty space below the tree's rows, then pick a menu
/// item with `keys` (the menu has the keyboard once it is open).
fn empty_space_menu(app: &App, keys: &str, cx: &mut TestAppContext) {
    app.draw(cx);
    let mut vcx = VisualTestContext::from_window(app.window, cx);
    let tree = vcx
        .debug_bounds("explorer-tree")
        .expect("the tree area is painted");
    let below_rows = gpui::point(tree.left() + gpui::px(20.), tree.bottom() - gpui::px(10.));
    vcx.simulate_mouse_move(below_rows, None, Modifiers::none());
    vcx.simulate_mouse_down(below_rows, gpui::MouseButton::Right, Modifiers::none());
    vcx.simulate_mouse_up(below_rows, gpui::MouseButton::Right, Modifiers::none());
    vcx.run_until_parked();
    app.draw(&mut vcx);
    for key in keys.split(' ') {
        vcx.simulate_keystrokes(key);
        vcx.run_until_parked();
        app.draw(&mut vcx);
    }
}

#[gpui::test]
fn the_empty_space_menu_creates_in_the_cave_root(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    std::fs::create_dir_all(dir.path().join(".granit/templates")).unwrap();
    std::fs::write(
        dir.path().join(".granit/templates/meeting.md"),
        "# {{ slug }}\n\n## Agenda\n",
    )
    .unwrap();
    let app = App::with_cave(dir.path(), cx);
    // Open a nested note: new entries still land in the root.
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_note(Some("b".into()), cx)
    });

    // "New note"
    empty_space_menu(&app, "down enter", cx);
    assert!(dir.path().join("untitled.md").exists());
    assert_eq!(app.active_note(cx).as_deref(), Some("untitled"));
    assert_eq!(
        renaming(&app, cx),
        Some((Entry::Note("untitled".into()), "untitled".into()))
    );
    type_and_commit(&app, "plain", cx);

    // "New note from template", then its only template.
    empty_space_menu(&app, "down down enter enter", cx);
    let raw = std::fs::read_to_string(dir.path().join("untitled.md")).unwrap();
    assert!(raw.ends_with("---\n# untitled\n\n## Agenda\n"), "{raw}");
    assert_eq!(
        renaming(&app, cx),
        Some((Entry::Note("untitled".into()), "untitled".into()))
    );
    type_and_commit(&app, "meeting-notes", cx);

    // "New folder"
    empty_space_menu(&app, "down down down enter", cx);
    assert!(dir.path().join("new-folder").is_dir());
    assert_eq!(
        renaming(&app, cx),
        Some((Entry::Folder("new-folder".into()), "new-folder".into()))
    );
    type_and_commit(&app, "projects", cx);
    assert_eq!(
        app.rows(cx),
        vec![
            "empty",
            "projects",
            "sub",
            "  b",
            "a",
            "meeting-notes",
            "plain"
        ]
    );

    // A right-click on a row still opens the row's own menu: "Rename" is
    // its first item.
    app.draw(cx);
    let mut vcx = VisualTestContext::from_window(app.window, cx);
    let row = vcx
        .debug_bounds("explorer-note-a")
        .expect("a is painted")
        .center();
    vcx.simulate_mouse_move(row, None, Modifiers::none());
    vcx.simulate_mouse_down(row, gpui::MouseButton::Right, Modifiers::none());
    vcx.simulate_mouse_up(row, gpui::MouseButton::Right, Modifiers::none());
    vcx.run_until_parked();
    app.draw(&mut vcx);
    vcx.simulate_keystrokes("down");
    vcx.simulate_keystrokes("enter");
    vcx.run_until_parked();
    assert_eq!(
        renaming(&app, cx),
        Some((Entry::Note("a".into()), "a".into()))
    );
    assert!(!dir.path().join("untitled.md").exists());
}

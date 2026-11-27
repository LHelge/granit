//! Explorer operations against a real cave: create, rename, move and
//! delete notes and folders.

use gpui::{AppContext as _, TestAppContext};

use super::{Entry, Explorer};
use crate::tests::{fixture_cave, App};

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

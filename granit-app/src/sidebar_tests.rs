//! Headless window tests of the sidebar: hiding and showing it, resizing it
//! by its edge, and its note count.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{point, px, AppContext as _, Modifiers, MouseButton, TestAppContext, VisualTestContext};
use granit_core::cave::Cave;
use granit_types::AppConfig;

use crate::config_ops::{ConfigEvent, ConfigEvents};
use crate::explorer::Entry;
use crate::host::Host;
use crate::tests::{fixture_cave, App};
use crate::workspace::{SIDEBAR_MIN_WIDTH, TOGGLE_SIDEBAR_KEY};

/// The config the cave has on disk.
fn saved_config(dir: &tempfile::TempDir) -> AppConfig {
    Cave::open(dir.path().to_path_buf())
        .unwrap()
        .load_config()
        .unwrap()
}

fn shown(app: &App, cx: &mut TestAppContext) -> bool {
    cx.read_entity(&app.workspace, |workspace, _| workspace.sidebar_shown())
}

fn note_count(app: &App, cx: &mut TestAppContext) -> usize {
    cx.read_entity(&app.workspace, |workspace, _| workspace.note_count())
}

fn press_toggle(app: &App, cx: &mut TestAppContext) {
    cx.simulate_keystrokes(app.window, TOGGLE_SIDEBAR_KEY);
    cx.run_until_parked();
    app.draw(cx);
}

#[gpui::test]
fn the_sidebar_toggles_and_remembers_it_in_the_cave(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    app.draw(cx);
    assert!(shown(&app, cx), "a new cave shows the sidebar");
    assert!(saved_config(&dir).sidebar.visible);

    press_toggle(&app, cx);
    assert!(!shown(&app, cx));
    assert!(
        !saved_config(&dir).sidebar.visible,
        "saved in sidebar.visible"
    );

    press_toggle(&app, cx);
    assert!(shown(&app, cx));
    assert!(saved_config(&dir).sidebar.visible);

    // Hiding it with the keyboard in its search box takes the focus back,
    // so the key shows it again.
    cx.update_window(app.window, |_, window, cx| {
        app.workspace
            .update(cx, |workspace, cx| workspace.focus_search(window, cx))
    })
    .unwrap();
    app.draw(cx);
    press_toggle(&app, cx);
    assert!(!shown(&app, cx));
    press_toggle(&app, cx);
    assert!(
        shown(&app, cx),
        "the key still works once the sidebar is gone"
    );

    // What the header's and the rail's buttons call; the next start keeps
    // it hidden.
    cx.update_window(app.window, |_, window, cx| {
        app.workspace
            .update(cx, |workspace, cx| workspace.toggle_sidebar(window, cx))
    })
    .unwrap();
    cx.run_until_parked();
    app.draw(cx);
    assert!(!saved_config(&dir).sidebar.visible);
    let reopened = App::with_cave(dir.path(), cx);
    assert!(!shown(&reopened, cx), "a hidden sidebar stays hidden");
    reopened.draw(cx);
}

fn search_notes(app: &App, cx: &mut TestAppContext) {
    cx.simulate_keystrokes(app.window, crate::workspace::SEARCH_NOTES_KEY);
    cx.run_until_parked();
    app.draw(cx);
}

#[gpui::test]
fn the_search_key_shows_a_hidden_sidebar(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    app.draw(cx);
    press_toggle(&app, cx);
    assert!(!shown(&app, cx));

    search_notes(&app, cx);
    assert!(shown(&app, cx), "the search box is drawn to take the keys");
    assert!(saved_config(&dir).sidebar.visible, "and that is remembered");
    assert_eq!(
        cx.read_entity(&app.workspace, |workspace, _| workspace.sidebar_tab()),
        crate::panels::SidebarTab::Search
    );

    // The keyboard is in the search box; the workspace's keys still work.
    press_toggle(&app, cx);
    assert!(!shown(&app, cx));
    press_toggle(&app, cx);
    assert!(shown(&app, cx));
}

#[gpui::test]
fn opening_a_cave_with_a_hidden_sidebar_keeps_the_keys_working(cx: &mut TestAppContext) {
    let hidden = fixture_cave();
    let other = fixture_cave();
    let app = App::with_cave(hidden.path(), cx);
    app.draw(cx);
    press_toggle(&app, cx);
    assert!(!saved_config(&hidden).sidebar.visible);

    app.update_workspace(cx, |workspace, cx| {
        workspace.open_cave(other.path().to_path_buf(), cx)
    });
    app.draw(cx);
    assert!(shown(&app, cx), "the other cave shows its sidebar");

    // With the keyboard in the sidebar's search box, open the cave whose
    // sidebar is hidden.
    search_notes(&app, cx);
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_cave(hidden.path().to_path_buf(), cx)
    });
    app.draw(cx);
    assert!(!shown(&app, cx));
    press_toggle(&app, cx);
    assert!(shown(&app, cx), "the toggle key still works");
}

#[gpui::test]
fn a_hidden_sidebar_stops_its_panel_from_refreshing(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    std::fs::write(dir.path().join("a.md"), "---\ntags: [one]\n---\n# A\n").unwrap();
    let app = App::with_cave(dir.path(), cx);
    app.update_workspace(cx, |workspace, cx| {
        workspace.show_tab(crate::panels::SidebarTab::Tags, cx)
    });
    let tags = cx.read_entity(&app.workspace, |workspace, _| workspace.tags().clone());
    let rows = |cx: &mut TestAppContext| cx.read_entity(&tags, |panel, cx| panel.visible_rows(cx));
    assert_eq!(rows(cx), ["one (1)"]);
    app.draw(cx);

    press_toggle(&app, cx);
    std::fs::write(dir.path().join("a.md"), "---\ntags: [two]\n---\n# A\n").unwrap();
    cx.update(|cx| crate::events::CaveEvents::emit(crate::events::CaveEvent::NotesChanged, cx));
    cx.run_until_parked();
    assert_eq!(rows(cx), ["one (1)"], "a hidden panel does not query");

    press_toggle(&app, cx);
    assert_eq!(rows(cx), ["two (1)"], "it reruns when shown again");
}

#[gpui::test]
fn without_a_cave_the_sidebar_is_always_shown(cx: &mut TestAppContext) {
    let app = App::start(
        crate::host::HostOptions::default(),
        crate::workspace::StartupCave::None,
        cx,
    );
    app.draw(cx);
    press_toggle(&app, cx);
    assert!(shown(&app, cx), "it holds the open-cave button");
}

#[gpui::test]
fn dragging_the_edge_resizes_the_sidebar_and_saves_the_width_once(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    assert_eq!(saved_config(&dir).sidebar.width, 320);
    app.draw(cx);
    let mut window = VisualTestContext::from_window(app.window, cx);
    let edge = px(SIDEBAR_MIN_WIDTH);
    let y = window.update(|window, _| window.viewport_size()).height / 2.;
    let saves = Rc::new(Cell::new(0));
    let _counting = window.update(|_, cx| {
        let saves = saves.clone();
        cx.subscribe(&ConfigEvents::entity(cx), move |_, _: &ConfigEvent, _| {
            saves.set(saves.get() + 1)
        })
    });

    window.simulate_mouse_move(point(edge - px(2.), y), None, Modifiers::default());
    window.simulate_mouse_down(
        point(edge - px(2.), y),
        MouseButton::Left,
        Modifiers::default(),
    );
    for x in [edge + px(20.), edge + px(80.)] {
        window.simulate_mouse_move(point(x, y), MouseButton::Left, Modifiers::default());
        app.draw(&mut window);
    }
    let dragging = |cx: &mut TestAppContext| {
        cx.read_entity(&app.workspace, |workspace, _| {
            workspace.sidebar_resize().is_dragging()
        })
    };
    assert!(dragging(&mut window));
    assert_eq!(saves.get(), 0, "nothing is saved while dragging");
    assert_eq!(saved_config(&dir).sidebar.width, 320);

    window.simulate_mouse_up(
        point(edge + px(80.), y),
        MouseButton::Left,
        Modifiers::default(),
    );
    app.draw(&mut window);
    assert!(!dragging(&mut window));
    assert_eq!(saves.get(), 1, "the width is saved once");
    assert_eq!(saved_config(&dir).sidebar.width, 400);
    assert_eq!(
        window.update(|_, cx| Host::core(cx).lock_config().sidebar.width),
        400
    );

    // A drag past the widest width stops there.
    let edge = px(400.);
    window.simulate_mouse_move(point(edge - px(2.), y), None, Modifiers::default());
    window.simulate_mouse_down(
        point(edge - px(2.), y),
        MouseButton::Left,
        Modifiers::default(),
    );
    for x in [edge + px(20.), px(900.)] {
        window.simulate_mouse_move(point(x, y), MouseButton::Left, Modifiers::default());
        app.draw(&mut window);
    }
    window.simulate_mouse_up(point(px(900.), y), MouseButton::Left, Modifiers::default());
    app.draw(&mut window);
    assert_eq!(saved_config(&dir).sidebar.width, 600);
}

#[gpui::test]
fn the_footer_counts_the_notes(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    assert_eq!(note_count(&app, cx), 2, "a and sub/b");

    app.update_explorer(cx, |explorer, window, cx| {
        explorer.new_note(None, window, cx)
    });
    assert_eq!(note_count(&app, cx), 3);
    app.draw(cx);

    app.update_explorer(cx, |explorer, _, cx| {
        explorer.cancel_rename(cx);
        explorer.delete(Entry::Note("untitled".into()), cx);
        explorer.delete(Entry::Folder("sub".into()), cx);
    });
    assert_eq!(note_count(&app, cx), 1);
    app.draw(cx);
}

#[test]
fn the_count_reads_as_the_tauri_app_does() {
    use crate::workspace::note_count_label;
    assert_eq!(note_count_label(0), "0 notes");
    assert_eq!(note_count_label(1), "1 note");
    assert_eq!(note_count_label(2), "2 notes");
}

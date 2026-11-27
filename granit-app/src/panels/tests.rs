//! Headless window tests of the sidebar panels against a fixture cave:
//! switching tabs, the todos (opening and toggling), the tags, the search
//! and the calendar of daily notes.

use std::path::Path;

use gpui::{AppContext as _, Context, Entity, EntityInputHandler as _, TestAppContext, Window};

use super::{CalendarPanel, SearchPanel, SidebarTab, TagsPanel, TemplatesPanel, TodosPanel};
use crate::events::{CaveEvent, CaveEvents};
use crate::host::Host;
use crate::tests::App;
use granit_core::calendar::{today_slug, Month};

const NOTE_A: &str =
    "---\ntags: [work, home]\n---\n# A\n\n- [ ] buy milk\n- [x] paid rent\nThe needle is here.\n";
const NOTE_B: &str = "---\ntags: [work]\n---\n# B\n- [ ] call Bob\nanother Needle\n";

/// A cave with two tagged notes with todos and one daily note.
fn panel_cave() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.md"), NOTE_A).unwrap();
    std::fs::create_dir_all(dir.path().join("sub")).unwrap();
    std::fs::write(dir.path().join("sub/b.md"), NOTE_B).unwrap();
    std::fs::create_dir_all(dir.path().join("Daily")).unwrap();
    std::fs::write(dir.path().join("Daily/2026-11-02.md"), "# Monday\n").unwrap();
    dir
}

fn read(dir: &Path, name: &str) -> String {
    std::fs::read_to_string(dir.join(name)).unwrap()
}

/// Run `f` on a panel with the window, then let effects settle.
fn update<T: 'static, R>(
    app: &App,
    panel: &Entity<T>,
    cx: &mut TestAppContext,
    f: impl FnOnce(&mut T, &mut Window, &mut Context<T>) -> R,
) -> R {
    let result = cx
        .update_window(app.window, |_, window, cx| {
            panel.update(cx, |panel, cx| f(panel, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
    result
}

fn show(app: &App, tab: SidebarTab, cx: &mut TestAppContext) {
    app.update_workspace(cx, |workspace, cx| workspace.show_tab(tab, cx));
}

fn todos(app: &App, cx: &mut TestAppContext) -> Entity<TodosPanel> {
    cx.read_entity(&app.workspace, |workspace, _| workspace.todos().clone())
}

fn tags(app: &App, cx: &mut TestAppContext) -> Entity<TagsPanel> {
    cx.read_entity(&app.workspace, |workspace, _| workspace.tags().clone())
}

fn search(app: &App, cx: &mut TestAppContext) -> Entity<SearchPanel> {
    cx.read_entity(&app.workspace, |workspace, _| workspace.search().clone())
}

fn calendar(app: &App, cx: &mut TestAppContext) -> Entity<CalendarPanel> {
    cx.read_entity(&app.workspace, |workspace, _| workspace.calendar().clone())
}

fn todo_rows(app: &App, cx: &mut TestAppContext) -> Vec<String> {
    let panel = todos(app, cx);
    cx.read_entity(&panel, |panel, cx| panel.visible_rows(cx))
}

fn search_rows(app: &App, cx: &mut TestAppContext) -> Vec<String> {
    let panel = search(app, cx);
    cx.read_entity(&panel, |panel, cx| panel.visible_rows(cx))
}

/// The caret offset in the open note's editor.
fn cursor(app: &App, cx: &mut TestAppContext) -> usize {
    let view = cx.read_entity(&app.workspace, |workspace, _| workspace.note_view().clone());
    let editor = cx
        .read_entity(&view, |view, _| view.editor().cloned())
        .expect("the editor is installed");
    cx.read_entity(&editor, |editor, _| editor.cursor())
}

/// Let the debounced search run.
fn settle_search(cx: &mut TestAppContext) {
    cx.executor().advance_clock(super::search::SEARCH_DELAY * 4);
    cx.run_until_parked();
}

#[gpui::test]
fn the_tab_bar_switches_the_sidebar_panel(cx: &mut TestAppContext) {
    let dir = panel_cave();
    let app = App::with_cave(dir.path(), cx);
    app.draw(cx);
    for tab in SidebarTab::ALL {
        show(&app, tab, cx);
        let shown = cx.read_entity(&app.workspace, |workspace, _| workspace.sidebar_tab());
        assert_eq!(shown, tab);
        app.draw(cx);
    }
}

#[gpui::test]
fn the_search_key_shows_the_search_panel(cx: &mut TestAppContext) {
    let dir = panel_cave();
    let app = App::with_cave(dir.path(), cx);
    app.draw(cx);
    cx.simulate_keystrokes(app.window, crate::workspace::SEARCH_NOTES_KEY);
    cx.run_until_parked();
    let shown = cx.read_entity(&app.workspace, |workspace, _| workspace.sidebar_tab());
    assert_eq!(shown, SidebarTab::Search);
    app.draw(cx);
}

#[gpui::test]
fn todos_list_open_tasks_by_note_and_open_a_todo_at_its_line(cx: &mut TestAppContext) {
    let dir = panel_cave();
    let app = App::with_cave(dir.path(), cx);
    show(&app, SidebarTab::Todos, cx);

    assert_eq!(
        todo_rows(&app, cx),
        ["a", "[ ] buy milk", "b", "[ ] call Bob", "Completed (1)"]
    );
    let panel = todos(&app, cx);
    update(&app, &panel, cx, |panel, _, cx| panel.click_row(4, cx));
    assert_eq!(
        todo_rows(&app, cx),
        [
            "a",
            "[ ] buy milk",
            "b",
            "[ ] call Bob",
            "Completed (1)",
            "a",
            "[x] paid rent"
        ]
    );
    app.draw(cx);

    update(&app, &panel, cx, |panel, _, cx| panel.click_row(3, cx));
    assert_eq!(app.active_note(cx).as_deref(), Some("b"));
    // `call Bob` is body line 1, after `# B\n`.
    assert_eq!(cursor(&app, cx), "# B\n".len());

    update(&app, &panel, cx, |panel, _, cx| panel.click_row(0, cx));
    assert_eq!(app.active_note(cx).as_deref(), Some("a"));
}

#[gpui::test]
fn toggling_a_todo_edits_its_note_and_refreshes_the_list(cx: &mut TestAppContext) {
    let dir = panel_cave();
    let app = App::with_cave(dir.path(), cx);
    show(&app, SidebarTab::Todos, cx);
    let panel = todos(&app, cx);

    update(&app, &panel, cx, |panel, _, cx| panel.click_checkbox(1, cx));

    let saved = read(dir.path(), "a.md");
    assert!(saved.starts_with("---\ntags: [work, home]\n"), "{saved}");
    assert!(saved.ends_with("---\n# A\n\n- [x] buy milk\n- [x] paid rent\nThe needle is here.\n"));
    assert_eq!(todo_rows(&app, cx), ["b", "[ ] call Bob", "Completed (2)"]);
}

#[gpui::test]
fn toggling_a_todo_of_the_open_note_keeps_its_pending_edits(cx: &mut TestAppContext) {
    let dir = panel_cave();
    let app = App::with_cave(dir.path(), cx);
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_note(Some("a".into()), cx)
    });
    show(&app, SidebarTab::Todos, cx);
    let panel = todos(&app, cx);

    // Two lines typed above the todos and not saved yet move them down.
    let view = cx.read_entity(&app.workspace, |workspace, _| workspace.note_view().clone());
    let editor = cx
        .read_entity(&view, |view, _| view.editor().cloned())
        .unwrap();
    update(&app, &editor, cx, |editor, window, cx| {
        editor.set_cursor("# A\n\n".len(), cx);
        editor.replace_text_in_range(None, "typed\n\n", window, cx);
    });
    update(&app, &panel, cx, |panel, _, cx| panel.click_checkbox(1, cx));

    let saved = read(dir.path(), "a.md");
    assert!(
        saved.ends_with("# A\n\ntyped\n\n- [x] buy milk\n- [x] paid rent\nThe needle is here.\n"),
        "{saved}"
    );
    let shown = app.shown_body(cx).unwrap();
    assert!(
        shown.contains("typed\n\n- [x] buy milk"),
        "the editor reloaded: {shown}"
    );
    let error = cx.read_entity(&panel, |panel, _| panel.error().cloned());
    assert_eq!(error, None);
}

#[gpui::test]
fn a_hidden_todos_panel_refreshes_when_shown(cx: &mut TestAppContext) {
    let dir = panel_cave();
    let app = App::with_cave(dir.path(), cx);
    show(&app, SidebarTab::Todos, cx);
    show(&app, SidebarTab::Notes, cx);

    // An agent tool adds a todo while the panel is hidden.
    std::fs::write(dir.path().join("sub/b.md"), "- [ ] new\n").unwrap();
    cx.update(|cx| CaveEvents::emit(CaveEvent::NotesChanged, cx));
    cx.run_until_parked();
    assert_eq!(todo_rows(&app, cx)[2], "b", "not rerun while hidden");
    assert_eq!(todo_rows(&app, cx)[3], "[ ] call Bob");

    show(&app, SidebarTab::Todos, cx);
    assert_eq!(
        todo_rows(&app, cx),
        ["a", "[ ] buy milk", "b", "[ ] new", "Completed (1)"]
    );
}

#[gpui::test]
fn tags_list_their_counts_and_a_selected_tag_lists_its_notes(cx: &mut TestAppContext) {
    let dir = panel_cave();
    let app = App::with_cave(dir.path(), cx);
    show(&app, SidebarTab::Tags, cx);
    let panel = tags(&app, cx);
    let rows = |cx: &mut TestAppContext| cx.read_entity(&panel, |panel, cx| panel.visible_rows(cx));

    assert_eq!(rows(cx), ["home (1)", "work (2)"]);
    update(&app, &panel, cx, |panel, _, cx| panel.click_row(1, cx));
    assert_eq!(rows(cx), ["home (1)", "work (2)", "  a", "  b"]);
    app.draw(cx);

    update(&app, &panel, cx, |panel, _, cx| panel.click_row(3, cx));
    assert_eq!(app.active_note(cx).as_deref(), Some("b"));

    // Another tag replaces the expanded one; a click on it folds it.
    update(&app, &panel, cx, |panel, _, cx| panel.click_row(0, cx));
    assert_eq!(rows(cx), ["home (1)", "  a", "work (2)"]);
    update(&app, &panel, cx, |panel, _, cx| panel.click_row(0, cx));
    assert_eq!(rows(cx), ["home (1)", "work (2)"]);

    // A tag added by a save shows up.
    std::fs::write(dir.path().join("sub/b.md"), "---\ntags: [new]\n---\n").unwrap();
    cx.update(|cx| CaveEvents::emit(CaveEvent::NotesChanged, cx));
    cx.run_until_parked();
    assert_eq!(rows(cx), ["home (1)", "new (1)", "work (1)"]);
}

#[gpui::test]
fn search_results_follow_the_query_and_open_the_note_at_the_match(cx: &mut TestAppContext) {
    let dir = panel_cave();
    let app = App::with_cave(dir.path(), cx);
    show(&app, SidebarTab::Search, cx);
    let panel = search(&app, cx);
    assert!(search_rows(&app, cx).is_empty());
    app.draw(cx);

    update(&app, &panel, cx, |panel, window, cx| {
        panel.set_query("needle", window, cx)
    });
    settle_search(cx);
    assert_eq!(
        search_rows(&app, cx),
        [
            "a (1)",
            "  The needle is here.",
            "b (1)",
            "  another Needle"
        ]
    );
    app.draw(cx);

    // Typing more narrows the results.
    update(&app, &panel, cx, |panel, window, cx| {
        panel.set_query("another needle", window, cx)
    });
    settle_search(cx);
    assert_eq!(search_rows(&app, cx), ["b (1)", "  another Needle"]);

    update(&app, &panel, cx, |panel, _, cx| panel.click_row(1, cx));
    assert_eq!(app.active_note(cx).as_deref(), Some("b"));
    assert_eq!(cursor(&app, cx), "# B\n- [ ] call Bob\n".len());

    // A save that adds a match reruns the query.
    std::fs::write(dir.path().join("a.md"), "Another needle\n").unwrap();
    cx.update(|cx| CaveEvents::emit(CaveEvent::NotesChanged, cx));
    settle_search(cx);
    assert_eq!(
        search_rows(&app, cx),
        ["a (1)", "  Another needle", "b (1)", "  another Needle"]
    );

    update(&app, &panel, cx, |panel, window, cx| {
        panel.set_query("", window, cx)
    });
    settle_search(cx);
    assert!(search_rows(&app, cx).is_empty());
}

#[gpui::test]
fn opening_a_note_from_a_panel_closes_the_settings(cx: &mut TestAppContext) {
    let dir = panel_cave();
    let app = App::with_cave(dir.path(), cx);
    show(&app, SidebarTab::Tags, cx);
    let workspace = app.workspace.clone();
    cx.update_window(app.window, |_, window, cx| {
        workspace.update(cx, |workspace, cx| workspace.toggle_settings(window, cx))
    })
    .unwrap();
    cx.run_until_parked();
    let open = |cx: &mut TestAppContext| {
        cx.read_entity(&app.workspace, |workspace, _| {
            workspace.settings().is_some()
        })
    };
    assert!(open(cx));

    let panel = tags(&app, cx);
    update(&app, &panel, cx, |panel, _, cx| panel.click_row(1, cx));
    update(&app, &panel, cx, |panel, _, cx| panel.click_row(2, cx));
    assert_eq!(app.active_note(cx).as_deref(), Some("a"));
    assert!(!open(cx), "the note replaces the settings");
    app.draw(cx);
}

#[gpui::test]
fn a_rerun_for_an_old_query_does_not_bring_its_results_back(cx: &mut TestAppContext) {
    let dir = panel_cave();
    let app = App::with_cave(dir.path(), cx);
    show(&app, SidebarTab::Search, cx);
    let panel = search(&app, cx);
    update(&app, &panel, cx, |panel, window, cx| {
        panel.set_query("needle", window, cx)
    });
    settle_search(cx);
    assert!(!search_rows(&app, cx).is_empty());

    // A cave change reruns "needle"; the box is cleared before it finishes.
    cx.update(|cx| CaveEvents::emit(CaveEvent::NotesChanged, cx));
    cx.run_until_parked();
    update(&app, &panel, cx, |panel, window, cx| {
        panel.set_query("", window, cx)
    });
    settle_search(cx);
    assert!(search_rows(&app, cx).is_empty());
}

#[gpui::test]
fn the_calendar_marks_daily_notes_and_opens_or_creates_a_day(cx: &mut TestAppContext) {
    let dir = panel_cave();
    let app = App::with_cave(dir.path(), cx);
    let panel = calendar(&app, cx);
    let shown = cx.read_entity(&panel, |panel, _| panel.month());
    assert_eq!(shown, Month::current(), "the calendar starts at this month");

    let november = Month::new(2026, 11).unwrap();
    update(&app, &panel, cx, |panel, _, cx| {
        panel.show_month(november, cx)
    });
    let marked = cx.read_entity(&panel, |panel, _| panel.marked_days());
    assert_eq!(marked, ["2026-11-02"]);
    app.draw(cx);

    // An existing day opens.
    update(&app, &panel, cx, |panel, _, cx| {
        panel.open_day("2026-11-02", cx)
    });
    assert_eq!(app.active_note(cx).as_deref(), Some("2026-11-02"));
    assert_eq!(app.shown_body(cx).as_deref(), Some("# Monday\n"));

    // A missing day is created in the daily-note folder and opens.
    update(&app, &panel, cx, |panel, _, cx| {
        panel.open_day("2026-11-20", cx)
    });
    assert_eq!(app.active_note(cx).as_deref(), Some("2026-11-20"));
    assert!(dir.path().join("Daily/2026-11-20.md").exists());
    let marked = cx.read_entity(&panel, |panel, _| panel.marked_days());
    assert_eq!(marked, ["2026-11-02", "2026-11-20"]);
    let explorer_rows = app.rows(cx);
    assert!(
        explorer_rows.contains(&"Daily".to_string()),
        "{explorer_rows:?}"
    );
    app.draw(cx);

    // A day that is no date is refused and the error shown.
    update(&app, &panel, cx, |panel, _, cx| {
        panel.open_day("../escape", cx)
    });
    let error = cx.read_entity(&panel, |panel, _| panel.error().cloned());
    assert!(error.is_some_and(|error| error.contains("invalid daily-note date")));
    assert_eq!(app.active_note(cx).as_deref(), Some("2026-11-20"));
    app.draw(cx);
}

#[gpui::test]
fn todays_daily_note_comes_from_the_template(cx: &mut TestAppContext) {
    let dir = panel_cave();
    std::fs::create_dir_all(dir.path().join(".granit/templates")).unwrap();
    std::fs::write(
        dir.path().join(".granit/templates/day.md"),
        "# {{ date }}\n",
    )
    .unwrap();
    let app = App::with_cave(dir.path(), cx);
    cx.update(|cx| {
        Host::core(cx).lock_config().daily_note_template_slug = Some("day".into());
    });

    let meta = cx
        .update(|cx| crate::cave_ops::open_daily_note(None, cx))
        .unwrap();
    cx.run_until_parked();

    let today = today_slug();
    assert_eq!(meta.slug, today);
    let body = cx
        .update(|cx| crate::cave_ops::read_note(&today, cx))
        .unwrap()
        .content;
    assert_eq!(body, format!("# {today}\n"));
    let panel = calendar(&app, cx);
    let marked = cx.read_entity(&panel, |panel, _| panel.marked_days());
    assert!(marked.contains(&today), "{marked:?}");
}

#[gpui::test]
fn the_daily_note_button_opens_todays_note_creating_it_once(cx: &mut TestAppContext) {
    let dir = panel_cave();
    std::fs::create_dir_all(dir.path().join(".granit/templates")).unwrap();
    std::fs::write(
        dir.path().join(".granit/templates/day.md"),
        "---\ntags: [journal]\n---\n# {{ date }}\n\n{% if day %}Back to [[{{ slug }}]]{% endif %}\n",
    )
    .unwrap();
    let app = App::with_cave(dir.path(), cx);
    cx.update(|cx| {
        crate::config_ops::update_config(cx, |config| {
            config.daily_note_template_slug = Some("day".into())
        })
    })
    .unwrap();
    cx.run_until_parked();
    let panel = calendar(&app, cx);
    // The calendar shows another month; the button comes back to this one.
    update(&app, &panel, cx, |panel, _, cx| {
        panel.show_month(Month::new(2001, 1).unwrap(), cx)
    });
    app.draw(cx);

    update(&app, &panel, cx, |panel, _, cx| panel.open_today(cx));

    let today = today_slug();
    assert_eq!(app.active_note(cx).as_deref(), Some(today.as_str()));
    assert_eq!(app.core_active_note(cx).as_deref(), Some(today.as_str()));
    let file = format!("Daily/{today}.md");
    let raw = read(dir.path(), &file);
    assert!(
        raw.contains("journal"),
        "the template's tags are copied: {raw}"
    );
    assert_eq!(
        app.shown_body(cx),
        Some(format!("# {today}\n\nBack to [[{today}]]\n")),
        "the body is the rendered template"
    );
    let shown = cx.read_entity(&panel, |panel, _| panel.month());
    assert_eq!(shown, Month::current());
    assert!(cx
        .read_entity(&panel, |panel, _| panel.marked_days())
        .contains(&today));
    app.draw(cx);

    // An existing day's note is opened as it is, never re-rendered.
    std::fs::write(dir.path().join(&file), "# kept\n").unwrap();
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_note(Some("a".into()), cx)
    });
    cx.update(|cx| CaveEvents::emit(CaveEvent::NotesChanged, cx));
    cx.run_until_parked();
    update(&app, &panel, cx, |panel, _, cx| panel.open_today(cx));
    assert_eq!(app.active_note(cx).as_deref(), Some(today.as_str()));
    assert_eq!(app.shown_body(cx).as_deref(), Some("# kept\n"));
    assert_eq!(read(dir.path(), &file), "# kept\n");
}

fn templates(app: &App, cx: &mut TestAppContext) -> Entity<TemplatesPanel> {
    cx.read_entity(&app.workspace, |workspace, _| workspace.templates().clone())
}

fn open_template(app: &App, cx: &mut TestAppContext) -> Option<String> {
    cx.read_entity(&app.workspace, |workspace, cx| {
        workspace.open_template_slug(cx)
    })
}

#[gpui::test]
fn templates_are_created_renamed_opened_and_deleted(cx: &mut TestAppContext) {
    let dir = panel_cave();
    std::fs::create_dir_all(dir.path().join(".granit/templates")).unwrap();
    std::fs::write(
        dir.path().join(".granit/templates/meeting.md"),
        "# Meeting\n",
    )
    .unwrap();
    let app = App::with_cave(dir.path(), cx);
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_note(Some("a".into()), cx)
    });
    show(&app, SidebarTab::Templates, cx);
    app.draw(cx);
    let panel = templates(&app, cx);
    assert_eq!(
        cx.read_entity(&panel, |panel, _| panel.slugs()),
        ["meeting"]
    );

    // A click opens the template; no note is active meanwhile.
    update(&app, &panel, cx, |panel, _, cx| {
        panel.open("meeting".into(), cx)
    });
    assert_eq!(open_template(&app, cx).as_deref(), Some("meeting"));
    assert_eq!(app.shown_body(cx).as_deref(), Some("# Meeting\n"));
    assert_eq!(app.active_note(cx), None);
    assert_eq!(app.core_active_note(cx), None);
    app.draw(cx);

    // New: created, opened and named inline.
    update(&app, &panel, cx, |panel, window, cx| {
        panel.new_template(window, cx)
    });
    assert!(dir.path().join(".granit/templates/untitled.md").exists());
    assert_eq!(open_template(&app, cx).as_deref(), Some("untitled"));
    let renaming = cx.read_entity(&panel, |panel, cx| panel.renaming(cx));
    assert_eq!(renaming, Some(("untitled".to_string(), "untitled".into())));
    app.draw(cx);
    update(&app, &panel, cx, |panel, window, cx| {
        panel.type_and_commit("standup", window, cx)
    });
    assert!(dir.path().join(".granit/templates/standup.md").exists());
    assert!(!dir.path().join(".granit/templates/untitled.md").exists());
    assert_eq!(
        cx.read_entity(&panel, |panel, _| panel.slugs()),
        ["meeting", "standup"]
    );
    assert_eq!(
        open_template(&app, cx).as_deref(),
        Some("standup"),
        "the editor follows"
    );

    // A name taken by another template is refused and shown.
    update(&app, &panel, cx, |panel, window, cx| {
        panel.start_rename("standup".into(), window, cx);
        panel.type_and_commit("meeting", window, cx)
    });
    assert!(cx.read_entity(&panel, |panel, _| panel.error().is_some()));
    assert!(dir.path().join(".granit/templates/standup.md").exists());

    // Deleting the open template closes it.
    update(&app, &panel, cx, |panel, _, cx| panel.delete("standup", cx));
    assert!(!dir.path().join(".granit/templates/standup.md").exists());
    assert_eq!(open_template(&app, cx), None);
    assert_eq!(app.shown_body(cx), None);
    assert_eq!(
        cx.read_entity(&panel, |panel, _| panel.slugs()),
        ["meeting"]
    );
    app.draw(cx);
}

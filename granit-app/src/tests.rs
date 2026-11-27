//! Headless window tests of the app shell: opening a cave, the explorer
//! tree, selection and the open-cave flow at startup.

use std::path::Path;

use gpui::{AnyWindowHandle, AppContext as _, Entity, TestAppContext, Window};
use gpui_kit::component::ActiveTheme as _;

use crate::events::{CaveEvent, CaveEvents};
use crate::explorer::Explorer;
use crate::host::{Host, HostOptions};
use crate::last_cave::LastCaveStore;
use crate::workspace::{open_main_window, StartupCave, Workspace};

const NOTE_A: &str = "---\nicon: Star\nfavorite: true\n---\n# A\n\nAlpha links [[b]].\n";
const NOTE_B: &str = "# B\n";

/// A cave with `a.md` (icon and favorite), `sub/b.md` and an empty folder.
pub(crate) fn fixture_cave() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.md"), NOTE_A).unwrap();
    std::fs::create_dir_all(dir.path().join("sub")).unwrap();
    std::fs::write(dir.path().join("sub/b.md"), NOTE_B).unwrap();
    std::fs::create_dir_all(dir.path().join("empty")).unwrap();
    dir
}

/// The path of the cave open in the core.
fn cave_path(cx: &mut TestAppContext) -> Option<std::path::PathBuf> {
    cx.update(|cx| Host::cave_path(cx))
}

/// The open main window and its root view.
pub(crate) struct App {
    pub window: AnyWindowHandle,
    pub workspace: Entity<Workspace>,
}

impl App {
    /// Start the app with `options`, open the main window for `startup` and
    /// let everything settle.
    pub fn start(options: HostOptions, startup: StartupCave, cx: &mut TestAppContext) -> Self {
        let (window, workspace) = cx.update(|cx| {
            gpui_kit::init(cx);
            Host::init(options, cx);
            crate::workspace::bind_keys(cx);
            open_main_window(startup, cx).unwrap()
        });
        cx.run_until_parked();
        Self { window, workspace }
    }

    /// Start with the cave at `path`, remembering nothing.
    pub fn with_cave(path: &Path, cx: &mut TestAppContext) -> Self {
        Self::start(
            HostOptions::default(),
            StartupCave::Open(path.to_path_buf()),
            cx,
        )
    }

    pub fn explorer(&self, cx: &mut TestAppContext) -> Entity<Explorer> {
        cx.read_entity(&self.workspace, |workspace, _| workspace.explorer().clone())
    }

    /// Run `f` on the explorer with the window, then let effects settle.
    pub fn update_explorer<R>(
        &self,
        cx: &mut TestAppContext,
        f: impl FnOnce(&mut Explorer, &mut Window, &mut gpui::Context<Explorer>) -> R,
    ) -> R {
        let explorer = self.explorer(cx);
        let result = cx
            .update_window(self.window, |_, window, cx| {
                explorer.update(cx, |explorer, cx| f(explorer, window, cx))
            })
            .unwrap();
        cx.run_until_parked();
        result
    }

    /// Run `f` on the workspace, then let effects settle.
    pub fn update_workspace<R>(
        &self,
        cx: &mut TestAppContext,
        f: impl FnOnce(&mut Workspace, &mut gpui::Context<Workspace>) -> R,
    ) -> R {
        let result = self.workspace.update(cx, f);
        cx.run_until_parked();
        result
    }

    pub fn rows(&self, cx: &mut TestAppContext) -> Vec<String> {
        let explorer = self.explorer(cx);
        cx.read_entity(&explorer, |explorer, cx| explorer.visible_rows(cx))
    }

    pub fn active_note(&self, cx: &mut TestAppContext) -> Option<String> {
        cx.read_entity(&self.workspace, |workspace, _| {
            workspace.active_note().map(str::to_string)
        })
    }

    /// The body the main area shows.
    pub fn shown_body(&self, cx: &mut TestAppContext) -> Option<String> {
        cx.read_entity(&self.workspace, |workspace, cx| {
            workspace.note_view().read(cx).body().map(str::to_string)
        })
    }

    /// The active note as the core (and so the agent's tools) sees it.
    pub fn core_active_note(&self, cx: &mut TestAppContext) -> Option<String> {
        cx.update(|cx| {
            Host::core(cx)
                .with_cave(|cave| Ok(cave.active_slug().map(str::to_string)))
                .unwrap()
        })
    }

    /// Paint the window once, which runs every view's render.
    pub fn draw(&self, cx: &mut TestAppContext) {
        cx.update_window(self.window, |_, window, cx| {
            let _ = window.draw(cx);
        })
        .unwrap();
    }
}

#[gpui::test]
fn opening_a_cave_shows_its_tree_and_seeds_granit_dir(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);

    assert_eq!(app.rows(cx), vec!["empty", "sub", "a"]);
    assert_eq!(
        cave_path(cx),
        Some(dir.path().to_path_buf()),
        "the cave is open in the core"
    );
    assert!(dir.path().join(".granit/config.yml").exists());
    assert!(dir.path().join(".granit/agent/system.md").exists());
    cx.update(|cx| {
        let name = &cx.theme().highlight_theme.name;
        assert!(name.starts_with("Catppuccin"), "{name}");
    });
    app.draw(cx);
}

#[gpui::test]
fn selecting_a_note_opens_it_and_tells_the_core(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);

    // Select the row of `a`, as a click or the arrow keys do.
    app.update_explorer(cx, |explorer, _, cx| explorer.select_row(2, cx));

    assert_eq!(app.active_note(cx).as_deref(), Some("a"));
    assert_eq!(app.core_active_note(cx).as_deref(), Some("a"));
    assert_eq!(
        app.shown_body(cx).as_deref(),
        Some("# A\n\nAlpha links [[b]].\n"),
        "the main area shows the body without frontmatter"
    );
    app.draw(cx);
}

#[gpui::test]
fn opening_a_nested_note_expands_and_selects_it(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);

    app.update_workspace(cx, |workspace, cx| {
        workspace.open_note(Some("b".into()), cx)
    });

    assert_eq!(app.rows(cx), vec!["empty", "sub", "  b", "a"]);
    let explorer = app.explorer(cx);
    let selected = cx.read_entity(&explorer, |explorer, cx| explorer.selected(cx));
    assert_eq!(selected, Some(crate::explorer::Entry::Note("b".into())));
    assert_eq!(app.shown_body(cx).as_deref(), Some(NOTE_B));
}

#[gpui::test]
fn changes_made_outside_the_explorer_refresh_the_tree(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_note(Some("a".into()), cx)
    });

    // An agent tool changes the cave through the core and reports it.
    cx.update(|cx| {
        Host::core(cx)
            .with_cave(|cave| {
                cave.create_note("from-agent", None, None)?;
                cave.delete_note("a")
            })
            .unwrap();
        CaveEvents::emit(CaveEvent::NotesChanged, cx);
    });
    cx.run_until_parked();

    assert_eq!(app.rows(cx), vec!["empty", "sub", "from-agent"]);
    assert_eq!(app.active_note(cx), None, "the deleted open note is closed");
    assert_eq!(app.shown_body(cx), None);
}

#[gpui::test]
fn a_cave_from_the_command_line_is_not_remembered(cx: &mut TestAppContext) {
    // `HostOptions` for a command-line cave has no store at all.
    assert!(HostOptions::from_environment(false).last_cave.is_none());

    let dir = fixture_cave();
    let state = tempfile::tempdir().unwrap();
    let store = LastCaveStore::at(state.path().join("last-cave.json"));
    store.save(Path::new("/the/real/cave")).unwrap();
    let _app = App::start(
        HostOptions::default(),
        StartupCave::Open(dir.path().to_path_buf()),
        cx,
    );
    assert_eq!(
        store.load().unwrap(),
        Some(Path::new("/the/real/cave").to_path_buf())
    );
}

#[gpui::test]
fn an_opened_cave_is_remembered(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let state = tempfile::tempdir().unwrap();
    let store = LastCaveStore::at(state.path().join("last-cave.json"));
    let options = HostOptions {
        last_cave: Some(store.clone()),
        embedding_models: None,
    };
    let _app = App::start(options, StartupCave::Open(dir.path().to_path_buf()), cx);
    assert_eq!(store.load().unwrap(), Some(dir.path().to_path_buf()));
}

#[gpui::test]
fn first_run_asks_for_a_cave_and_opens_the_picked_folder(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let state = tempfile::tempdir().unwrap();
    let store = LastCaveStore::at(state.path().join("last-cave.json"));
    let options = HostOptions {
        last_cave: Some(store.clone()),
        embedding_models: None,
    };
    let startup = cx.update(|cx| {
        gpui_kit::init(cx);
        Host::init(options, cx);
        crate::startup_cave(None, cx)
    });
    assert_eq!(startup, StartupCave::Pick, "nothing is remembered yet");

    let (_, workspace) = cx.update(|cx| open_main_window(startup, cx).unwrap());
    cx.run_until_parked();
    assert!(cx.did_prompt_for_paths(), "the folder picker is shown");
    let picked = dir.path().to_path_buf();
    cx.simulate_path_prompt_response(move |options| {
        assert!(options.directories && !options.files);
        Some(vec![picked])
    });
    cx.run_until_parked();

    assert_eq!(cave_path(cx), Some(dir.path().to_path_buf()));
    assert_eq!(store.load().unwrap(), Some(dir.path().to_path_buf()));
    let explorer = cx.read_entity(&workspace, |workspace, _| workspace.explorer().clone());
    let rows = cx.read_entity(&explorer, |explorer, cx| explorer.visible_rows(cx));
    assert_eq!(rows, vec!["empty", "sub", "a"]);

    // The next launch reopens it.
    let next = cx.update(|cx| crate::startup_cave(None, cx));
    assert_eq!(next, StartupCave::Open(dir.path().to_path_buf()));
}

#[gpui::test]
fn a_remembered_cave_that_is_gone_is_forgotten(cx: &mut TestAppContext) {
    let state = tempfile::tempdir().unwrap();
    let store = LastCaveStore::at(state.path().join("last-cave.json"));
    store.save(&state.path().join("deleted-cave")).unwrap();
    let options = HostOptions {
        last_cave: Some(store.clone()),
        embedding_models: None,
    };
    let startup = cx.update(|cx| {
        Host::init(options, cx);
        crate::startup_cave(None, cx)
    });
    assert_eq!(startup, StartupCave::Pick);
    assert_eq!(store.load().unwrap(), None);
}

#[gpui::test]
fn a_cave_that_fails_to_open_is_reported_and_not_remembered(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    std::fs::write(dir.path().join("dup.md"), "a").unwrap();
    std::fs::write(dir.path().join("sub/dup.md"), "b").unwrap();
    let state = tempfile::tempdir().unwrap();
    let store = LastCaveStore::at(state.path().join("last-cave.json"));
    let options = HostOptions {
        last_cave: Some(store.clone()),
        embedding_models: None,
    };

    let app = App::start(options, StartupCave::Open(dir.path().to_path_buf()), cx);

    assert_eq!(cave_path(cx), None);
    assert_eq!(store.load().unwrap(), None);
    assert!(app.rows(cx).is_empty());
    let failed = cx.read_entity(&app.workspace, |workspace, _| workspace.failure());
    assert!(
        failed.is_some_and(|message| message.contains("dup")),
        "the error names the duplicate"
    );
    app.draw(cx);
}

#[gpui::test]
fn a_failed_switch_keeps_the_open_cave(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_note(Some("a".into()), cx)
    });

    let missing = dir.path().join("no-such-cave");
    app.update_workspace(cx, |workspace, cx| workspace.open_cave(missing, cx));

    assert_eq!(cave_path(cx), Some(dir.path().to_path_buf()));
    assert_eq!(app.active_note(cx).as_deref(), Some("a"));
    assert_eq!(app.rows(cx), vec!["empty", "sub", "a"]);
    let failed = cx.read_entity(&app.workspace, |workspace, _| workspace.failure());
    assert!(failed.is_some());
    app.draw(cx);
}

#[gpui::test]
fn the_open_cave_key_works_in_a_fresh_window(cx: &mut TestAppContext) {
    let app = App::start(HostOptions::default(), StartupCave::None, cx);
    app.draw(cx);
    assert!(!cx.did_prompt_for_paths());

    // Nothing has been clicked; the workspace itself holds focus.
    cx.simulate_keystrokes(app.window, crate::workspace::OPEN_CAVE_KEY);
    cx.run_until_parked();

    assert!(cx.did_prompt_for_paths(), "the folder picker is shown");
}

#[gpui::test]
fn the_quit_key_parses(cx: &mut TestAppContext) {
    // `KeyBinding::new` panics on a keystroke it cannot parse.
    cx.update(|cx| {
        cx.bind_keys([gpui::KeyBinding::new(
            crate::workspace::QUIT_KEY,
            crate::Quit,
            None,
        )])
    });
}

#[gpui::test]
fn note_rows_show_their_icon_and_favorite_star(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_note(Some("b".into()), cx)
    });

    let explorer = app.explorer(cx);
    let decorations = cx.read_entity(&explorer, |explorer, cx| explorer.note_decorations(cx));
    let shown: Vec<(&str, &str, bool)> = decorations
        .iter()
        .map(|(slug, decoration)| (slug.as_str(), decoration.icon.as_ref(), decoration.favorite))
        .collect();
    assert_eq!(
        shown,
        vec![
            ("b", "note-icons/FileText.svg", false),
            ("a", "note-icons/Star.svg", true),
        ]
    );
}

#[gpui::test]
fn no_cave_at_startup_paints_the_open_prompt(cx: &mut TestAppContext) {
    let app = App::start(HostOptions::default(), StartupCave::None, cx);
    assert!(!cx.did_prompt_for_paths());
    assert!(app.rows(cx).is_empty());
    app.draw(cx);
}

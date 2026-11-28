//! Headless window tests of the backup UI: the settings' Backup page asks
//! before deleting a snapshot or replacing the cave, and the no-cave screen
//! offers a disaster restore.
//!
//! No backend runs here: requests go to a local port nothing listens on,
//! so they fail at once and the tests see the errors the UI shows, or to
//! one that never answers ([`HangingBackend`]) while a test looks at a
//! request in flight. The
//! flows themselves are tested against a fake backend in
//! `granit_core::backup::run`.

use std::time::Duration;

use gpui::{AppContext as _, Entity, EntityInputHandler as _, TestAppContext};
use granit_core::backup::BackupInfo;
use granit_types::{RestoreStage, RestoreTarget};

use crate::backup_ops::{BackupEvent, BackupOps, RestoreFailed, RestoreOutcome, Running};
use crate::config_ops;
use crate::disaster_restore::DisasterRestore;
use crate::host::{Host, HostOptions};
use crate::settings::backup::{BackupPanel, DELETE_ANSWER, REPLACE_ANSWER};
use crate::settings::fields::probe;
use crate::tests::{fixture_cave, App};
use crate::workspace::StartupCave;

/// The Backup page's index in the settings.
const BACKUP_PAGE: usize = 4;

/// A backend URL that refuses connections: a port that was just free.
fn unreachable_url() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    format!("http://127.0.0.1:{port}")
}

/// A backend that takes connections but never answers: the kernel completes
/// the handshake through the listen backlog, nothing accepts, so a request
/// stays in flight until the listener is dropped (which resets it).
struct HangingBackend(std::net::TcpListener);

impl HangingBackend {
    fn new() -> Self {
        Self(std::net::TcpListener::bind("127.0.0.1:0").unwrap())
    }

    fn url(&self) -> String {
        format!("http://{}", self.0.local_addr().unwrap())
    }
}

/// A complete snapshot record, as the backend lists it.
fn snapshot() -> BackupInfo {
    serde_json::from_value(serde_json::json!({
        "id": "00000000-0000-0000-0000-000000000001",
        "cave_name": "cave",
        "size_bytes": 2048,
        "sha256_hex": "ab",
        "state": "complete",
        "created_at": "2026-11-27T10:00:00Z",
        "completed_at": "2026-11-27T10:00:01Z",
    }))
    .unwrap()
}

/// Let tokio work finish, until `done` holds (real time passes). Call
/// `cx.executor().allow_parking()` before starting the work: a tokio thread
/// that wakes a GPUI task is outside the deterministic test scheduler.
fn wait_until(cx: &mut TestAppContext, done: impl Fn(&mut TestAppContext) -> bool) {
    for _ in 0..500 {
        cx.run_until_parked();
        if done(cx) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out");
}

/// A cave whose backup connection points at nothing, with the Backup page
/// open and drawn, and the snapshot list (which failed) replaced by one
/// complete snapshot.
fn backup_page(dir: &tempfile::TempDir, cx: &mut TestAppContext) -> (App, Entity<BackupPanel>) {
    let app = App::with_cave(dir.path(), cx);
    cx.executor().allow_parking();
    let url = unreachable_url();
    cx.update(|cx| {
        config_ops::update_config(cx, |config| {
            config.backup.backend_url = url;
            config.backup.api_key = "grnt_test".into();
        })
    })
    .unwrap();
    cx.update_window(app.window, |_, window, cx| {
        app.workspace
            .update(cx, |workspace, cx| workspace.toggle_settings(window, cx))
    })
    .unwrap();
    let settings = cx
        .read_entity(&app.workspace, |workspace, _| workspace.settings().cloned())
        .expect("the settings are open");
    settings.update(cx, |view, cx| view.show_page(BACKUP_PAGE, cx));
    cx.run_until_parked();
    app.draw(cx);
    let panel = cx.read_entity(&settings, |view, _| view.backup().clone());

    // Opening the settings lists the snapshots; nothing answers.
    wait_until(cx, |cx| {
        cx.read_entity(&panel, |panel, _| panel.list_error().is_some())
    });
    let error = cx.read_entity(&panel, |panel, _| panel.list_error().cloned().unwrap());
    assert!(error.contains("Could not list snapshots"), "{error}");

    panel.update(cx, |panel, cx| panel.set_snapshots(vec![snapshot()], cx));
    app.draw(cx);
    (app, panel)
}

/// Open the restore flow of [`snapshot`], as its Restore… button does.
fn choose_restore(app: &App, panel: &Entity<BackupPanel>, cx: &mut TestAppContext) {
    cx.update_window(app.window, |_, window, cx| {
        panel.update(cx, |panel, cx| panel.choose_restore(snapshot(), window, cx))
    })
    .unwrap();
    cx.run_until_parked();
}

#[gpui::test]
fn typed_credentials_list_the_snapshots_once_committed(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    cx.executor().allow_parking();
    let url = unreachable_url();
    cx.update(|cx| config_ops::update_config(cx, |config| config.backup.backend_url = url))
        .unwrap();
    cx.update_window(app.window, |_, window, cx| {
        app.workspace
            .update(cx, |workspace, cx| workspace.toggle_settings(window, cx))
    })
    .unwrap();
    let settings = cx
        .read_entity(&app.workspace, |workspace, _| workspace.settings().cloned())
        .unwrap();
    settings.update(cx, |view, cx| view.show_page(BACKUP_PAGE, cx));
    cx.run_until_parked();
    app.draw(cx);
    let panel = cx.read_entity(&settings, |view, _| view.backup().clone());
    let idle = |cx: &mut TestAppContext| {
        cx.read_entity(&panel, |panel, _| {
            !panel.is_loading() && panel.list_error().is_none() && panel.snapshots().is_none()
        })
    };
    assert!(idle(cx), "no API key, no listing");

    // Typing the key lists nothing until the typing pauses.
    let key = probe::input("backup-api-key").expect("the API key field is drawn");
    for text in ["g", "gr", "grnt_test"] {
        cx.update_window(app.window, |_, window, cx| {
            key.update(cx, |input, cx| input.replace_all(text, window, cx))
        })
        .unwrap();
        cx.run_until_parked();
        app.draw(cx);
        assert!(idle(cx), "not listed while typing {text:?}");
    }

    cx.executor().advance_clock(config_ops::COMMIT_DELAY * 2);
    cx.run_until_parked();
    wait_until(cx, |cx| {
        cx.read_entity(&panel, |panel, _| panel.list_error().is_some())
    });
    app.draw(cx);
}

#[gpui::test]
fn a_wrong_passphrase_keeps_the_passphrase_prompt(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let (app, panel) = backup_page(&dir, cx);
    choose_restore(&app, &panel, cx);
    let fail = |needs_passphrase: bool, cx: &mut TestAppContext| {
        cx.update(|cx| {
            BackupOps::emit(
                BackupEvent::RestoreFailed(RestoreFailed {
                    message: "Decryption failed".into(),
                    needs_passphrase,
                }),
                cx,
            )
        });
        cx.run_until_parked();
    };

    // The cached key does not fit: the prompt shows, with no error.
    panel.update(cx, |panel, cx| {
        panel.begin_restore(RestoreTarget::CurrentCave, None, cx)
    });
    fail(true, cx);
    assert!(cx.read_entity(&panel, |panel, _| panel.asks_for_passphrase()));
    assert!(cx.read_entity(&panel, |panel, _| {
        panel
            .restore_state()
            .is_some_and(|(_, error)| error.is_none())
    }));
    app.draw(cx);

    // A wrong passphrase keeps the prompt and shows why.
    panel.update(cx, |panel, cx| {
        panel.begin_restore(RestoreTarget::CurrentCave, Some("wrong".into()), cx)
    });
    fail(false, cx);
    assert!(cx.read_entity(&panel, |panel, _| panel.asks_for_passphrase()));
    assert!(cx.read_entity(&panel, |panel, _| {
        panel
            .restore_state()
            .is_some_and(|(_, error)| error.is_some())
    }));
    app.draw(cx);

    // Choosing a snapshot again starts over with an empty passphrase.
    let input = cx.read_entity(&panel, |panel, _| panel.restore_passphrase().clone());
    cx.update_window(app.window, |_, window, cx| {
        input.update(cx, |input, cx| input.replace_all("typed", window, cx))
    })
    .unwrap();
    choose_restore(&app, &panel, cx);
    assert_eq!(cx.read_entity(&input, |input, _| input.value()), "");
    assert!(!cx.read_entity(&panel, |panel, _| panel.asks_for_passphrase()));
}

#[gpui::test]
fn no_cave_opens_while_a_restore_runs(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let other = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    cx.update(|cx| BackupOps::set_running(Some(Running::Restore(RestoreStage::Downloading)), cx));

    let path = other.path().to_path_buf();
    app.update_workspace(cx, |workspace, cx| workspace.open_cave(path, cx));

    assert_eq!(
        cx.update(|cx| Host::cave_path(cx)),
        Some(dir.path().to_path_buf())
    );
    let failure = cx.read_entity(&app.workspace, |workspace, _| workspace.failure());
    assert!(
        failure.is_some_and(|message| message.contains("a restore is running")),
        "the refusal is shown"
    );
    app.draw(cx);

    cx.update(|cx| BackupOps::set_running(None, cx));
    let path = other.path().to_path_buf();
    app.update_workspace(cx, |workspace, cx| workspace.open_cave(path, cx));
    assert_eq!(
        cx.update(|cx| Host::cave_path(cx)),
        Some(other.path().to_path_buf())
    );
}

#[gpui::test]
fn a_cave_that_fails_to_open_shows_its_error_in_the_disaster_restore(cx: &mut TestAppContext) {
    let app = App::start(HostOptions::default(), StartupCave::None, cx);
    cx.update_window(app.window, |_, window, cx| {
        app.workspace.update(cx, |workspace, cx| {
            workspace.show_disaster_restore(window, cx)
        })
    })
    .unwrap();
    cx.run_until_parked();
    let view = cx
        .read_entity(&app.workspace, |workspace, _| {
            workspace.disaster_restore().cloned()
        })
        .unwrap();

    // What the workspace does once a restore is done, with a cave that does
    // not open.
    let gone = tempfile::tempdir().unwrap().path().join("gone");
    app.update_workspace(cx, |workspace, cx| workspace.open_cave(gone, cx));

    let error = cx.read_entity(&view, |view, _| view.error().cloned());
    assert!(
        error.is_some_and(|error| error.contains("Could not open")),
        "the view shows why"
    );
    assert!(cx.read_entity(&app.workspace, |workspace, _| workspace
        .disaster_restore()
        .is_some()));
    app.draw(cx);
}

#[gpui::test]
fn the_api_key_is_masked(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let (app, _) = backup_page(&dir, cx);
    let key = probe::input("backup-api-key").expect("the API key field is drawn");
    assert!(cx.read_entity(&key, |input, _| input.presentation().is_masked()));
    assert_eq!(cx.read_entity(&key, |input, _| input.value()), "grnt_test");
    let url = probe::input("backup-url").expect("the URL field is drawn");
    assert!(!cx.read_entity(&url, |input, _| input.presentation().is_masked()));
    app.draw(cx);
}

#[gpui::test]
fn deleting_a_snapshot_asks_first(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let (app, panel) = backup_page(&dir, cx);
    let ask = |cx: &mut TestAppContext| {
        cx.update_window(app.window, |_, window, cx| {
            panel.update(cx, |panel, cx| panel.request_delete(snapshot(), window, cx))
        })
        .unwrap();
        cx.run_until_parked();
    };

    // Cancelling deletes nothing.
    ask(cx);
    assert!(cx.has_pending_prompt(), "the delete asks first");
    cx.simulate_prompt_answer("Cancel");
    cx.run_until_parked();
    assert!(!cx.read_entity(&panel, |panel, _| panel.is_deleting()));
    assert!(cx.read_entity(&panel, |panel, _| panel.delete_error().is_none()));
    assert_eq!(
        cx.read_entity(&panel, |panel, _| panel.snapshots().map(<[_]>::len)),
        Some(1)
    );

    // Confirming sends the delete, which stays in flight while the backend
    // does not answer, then fails and says so.
    let backend = HangingBackend::new();
    let url = backend.url();
    cx.update(|cx| Host::core(cx).lock_config().backup.backend_url = url);
    ask(cx);
    cx.simulate_prompt_answer(DELETE_ANSWER);
    cx.run_until_parked();
    assert!(cx.read_entity(&panel, |panel, _| panel.is_deleting()));
    app.draw(cx);
    drop(backend);
    wait_until(cx, |cx| {
        cx.read_entity(&panel, |panel, _| panel.delete_error().is_some())
    });
    let error = cx.read_entity(&panel, |panel, _| panel.delete_error().cloned().unwrap());
    assert!(error.contains("Could not delete snapshot"), "{error}");
    app.draw(cx);
}

#[gpui::test]
fn replacing_the_cave_asks_first_and_a_failed_restore_keeps_it(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let (app, panel) = backup_page(&dir, cx);
    choose_restore(&app, &panel, cx);
    app.draw(cx);
    let ask = |cx: &mut TestAppContext| {
        cx.update_window(app.window, |_, window, cx| {
            panel.update(cx, |panel, cx| panel.request_replace(window, cx))
        })
        .unwrap();
        cx.run_until_parked();
    };

    // Cancelling restores nothing.
    ask(cx);
    assert!(cx.has_pending_prompt(), "replacing the cave asks first");
    cx.simulate_prompt_answer("Cancel");
    cx.run_until_parked();
    assert_eq!(cx.update(|cx| BackupOps::running(cx)), None);

    // Confirming runs the restore, which fails before touching the cave.
    ask(cx);
    cx.simulate_prompt_answer(REPLACE_ANSWER);
    cx.run_until_parked();
    wait_until(cx, |cx| {
        cx.read_entity(&panel, |panel, _| {
            panel
                .restore_state()
                .is_some_and(|(_, error)| error.is_some())
        })
    });
    assert_eq!(cx.update(|cx| BackupOps::running(cx)), None);
    assert_eq!(
        cx.update(|cx| Host::cave_path(cx)),
        Some(dir.path().to_path_buf()),
        "the cave is still open"
    );
    assert!(dir.path().join("a.md").is_file(), "and untouched");
    app.draw(cx);
}

/// Replace the open note's text in its editor, as typing does.
fn edit_note(app: &App, text: &str, cx: &mut TestAppContext) {
    let view = cx.read_entity(&app.workspace, |workspace, _| workspace.note_view().clone());
    let editor = cx
        .read_entity(&view, |view, _| view.editor().cloned())
        .expect("the editor is installed");
    cx.update_window(app.window, |_, window, cx| {
        editor.update(cx, |editor, cx| {
            let len = editor.text().len();
            editor.replace_text_in_range(Some(0..len), text, window, cx);
        })
    })
    .unwrap();
    cx.run_until_parked();
}

#[gpui::test]
fn the_note_is_locked_while_a_restore_replaces_the_cave(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    cx.executor().allow_parking();
    let backend = HangingBackend::new();
    let url = backend.url();
    cx.update(|cx| {
        config_ops::update_config(cx, |config| {
            config.backup.backend_url = url;
            config.backup.api_key = "grnt_test".into();
        })
    })
    .unwrap();
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_note(Some("a".into()), cx)
    });
    app.draw(cx);
    edit_note(&app, "# A\n\nTyped before the restore.\n", cx);
    let view = cx.read_entity(&app.workspace, |workspace, _| workspace.note_view().clone());
    let locked = |cx: &mut TestAppContext| cx.read_entity(&view, |view, _| view.locked().cloned());

    // Starting the restore writes the edits and locks the note while the
    // snapshot downloads (the backend never answers).
    app.update_workspace(cx, |workspace, cx| {
        workspace.restore_backup(
            granit_core::backup::RestoreRequest {
                id: snapshot().id,
                target: RestoreTarget::CurrentCave,
                passphrase: None,
                credentials: None,
            },
            cx,
        )
    });
    assert!(std::fs::read_to_string(dir.path().join("a.md"))
        .unwrap()
        .contains("Typed before the restore."));
    assert_eq!(
        cx.update(|cx| BackupOps::running(cx)),
        Some(Running::Restore(RestoreStage::Downloading))
    );
    assert!(locked(cx).is_some_and(|reason| reason.contains("being restored")));
    app.draw(cx);

    // Typing reaches nothing: the editor is not drawn.
    let editor = cx
        .read_entity(&view, |view, _| view.editor().cloned())
        .unwrap();
    cx.update_window(app.window, |_, window, cx| {
        editor.update(cx, |editor, cx| editor.focus(window, cx))
    })
    .unwrap();
    app.draw(cx);
    cx.simulate_input(app.window, "typed while restoring");
    cx.run_until_parked();
    app.draw(cx);
    assert_eq!(
        app.shown_body(cx).as_deref(),
        Some("# A\n\nTyped before the restore.\n")
    );

    // A restore that fails before closing the cave unlocks the note.
    drop(backend);
    wait_until(cx, |cx| {
        cx.update(|cx| BackupOps::running(cx)).is_none() && locked(cx).is_none()
    });
    assert_eq!(
        cx.update(|cx| Host::cave_path(cx)),
        Some(dir.path().to_path_buf())
    );
    assert_eq!(app.active_note(cx).as_deref(), Some("a"));
    app.draw(cx);

    // An in-place restore that got past closing the cave: the note holds
    // edits that can no longer be saved. They belong to the replaced cave,
    // and do not keep the restored one from opening.
    edit_note(&app, "# A\n\nLost with the replaced cave.\n", cx);
    cx.update(|cx| *Host::core(cx).lock_cave() = None);
    app.update_workspace(cx, |workspace, cx| {
        workspace.finish_restore(
            true,
            RestoreOutcome {
                open: Some(dir.path().to_path_buf()),
                error: None,
            },
            cx,
        )
    });
    wait_until(cx, |cx| cx.update(|cx| Host::cave_path(cx)).is_some());
    assert_eq!(
        cx.update(|cx| Host::cave_path(cx)),
        Some(dir.path().to_path_buf())
    );
    assert_eq!(
        cx.read_entity(&app.workspace, |workspace, _| workspace.failure()),
        None
    );
    assert!(locked(cx).is_none());
    assert!(!std::fs::read_to_string(dir.path().join("a.md"))
        .unwrap()
        .contains("Lost with the replaced cave."));
    app.draw(cx);

    // And the reopened cave switches like any other.
    let other = fixture_cave();
    let path = other.path().to_path_buf();
    app.update_workspace(cx, |workspace, cx| workspace.open_cave(path, cx));
    wait_until(cx, |cx| {
        cx.update(|cx| Host::cave_path(cx)) == Some(other.path().to_path_buf())
    });
}

/// Ask for [`snapshot`] to be restored over the open cave.
fn restore_in_place(app: &App, cx: &mut TestAppContext) {
    app.update_workspace(cx, |workspace, cx| {
        workspace.restore_backup(
            granit_core::backup::RestoreRequest {
                id: snapshot().id,
                target: RestoreTarget::CurrentCave,
                passphrase: None,
                credentials: None,
            },
            cx,
        )
    });
}

#[gpui::test]
fn a_restore_over_the_cave_stops_a_running_reply_first(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    cx.executor().allow_parking();
    let backend = HangingBackend::new();
    let url = backend.url();
    cx.update(|cx| {
        config_ops::update_config(cx, |config| {
            config.backup.backend_url = url;
            config.backup.api_key = "grnt_test".into();
        })
    })
    .unwrap();
    let chat = cx.read_entity(&app.workspace, |workspace, _| workspace.chat().clone());
    let reply = |cx: &mut TestAppContext| {
        chat.update(cx, |chat, cx| {
            chat.run_stream(std::future::pending(), "hi".into(), 0, cx)
        })
    };
    let streaming = |cx: &mut TestAppContext| cx.read_entity(&chat, |chat, _| chat.is_streaming());

    // A restore refused for edits that cannot be written leaves the reply.
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_note(Some("b".into()), cx)
    });
    app.draw(cx);
    edit_note(&app, "# B\n\nCannot be written.\n", cx);
    std::fs::rename(dir.path().join("sub"), dir.path().join("sub.kept")).unwrap();
    std::fs::write(dir.path().join("sub"), "").unwrap();
    reply(cx);
    restore_in_place(&app, cx);
    assert_eq!(cx.update(|cx| BackupOps::running(cx)), None);
    assert!(streaming(cx), "a refused restore stops nothing");

    // Once they can be written the restore starts, and stops the reply
    // while its cave is still the one installed.
    std::fs::remove_file(dir.path().join("sub")).unwrap();
    std::fs::rename(dir.path().join("sub.kept"), dir.path().join("sub")).unwrap();
    restore_in_place(&app, cx);
    assert_eq!(
        cx.update(|cx| BackupOps::running(cx)),
        Some(Running::Restore(RestoreStage::Downloading))
    );
    assert!(!streaming(cx));
    assert_eq!(
        cx.read_entity(&chat, |chat, _| chat.stopped_in()),
        Some(Some(dir.path().to_path_buf()))
    );

    drop(backend);
    wait_until(cx, |cx| cx.update(|cx| BackupOps::running(cx)).is_none());
}

#[gpui::test]
fn the_no_cave_screen_offers_a_disaster_restore(cx: &mut TestAppContext) {
    let app = App::start(HostOptions::default(), StartupCave::None, cx);
    cx.executor().allow_parking();
    cx.update_window(app.window, |_, window, cx| {
        app.workspace.update(cx, |workspace, cx| {
            workspace.show_disaster_restore(window, cx)
        })
    })
    .unwrap();
    cx.run_until_parked();
    app.draw(cx);
    let view: Entity<DisasterRestore> = cx
        .read_entity(&app.workspace, |workspace, _| {
            workspace.disaster_restore().cloned()
        })
        .expect("the disaster restore is shown");

    // The connection is typed in and used for the listing, never saved.
    let [url, key, passphrase] = cx.read_entity(&view, |view, _| view.inputs());
    assert!(cx.read_entity(&key, |input, _| input.presentation().is_masked()));
    assert!(cx.read_entity(&passphrase, |input, _| input.presentation().is_masked()));
    let backend = unreachable_url();
    cx.update_window(app.window, |_, window, cx| {
        url.update(cx, |input, cx| input.replace_all(&backend, window, cx));
        key.update(cx, |input, cx| input.replace_all("grnt_test", window, cx));
    })
    .unwrap();
    view.update(cx, |view, cx| view.list(cx));
    assert!(cx.read_entity(&view, |view, _| view.is_listing()));
    wait_until(cx, |cx| cx.read_entity(&view, |view, _| !view.is_listing()));
    let error = cx.read_entity(&view, |view, _| view.error().cloned());
    assert!(
        error.is_some_and(|error| error.contains("Could not list snapshots")),
        "the failure is shown"
    );
    assert_eq!(
        cx.read_entity(&view, |view, _| view.snapshots().map(<[_]>::len)),
        None
    );
    app.draw(cx);

    // A snapshot leads to the target folder and passphrase step.
    view.update(cx, |view, cx| {
        view.select(Some(snapshot()), cx);
        view.set_target(std::env::temp_dir().join("never-restored"), cx);
    });
    app.draw(cx);
    assert_eq!(cx.update(|cx| Host::cave_path(cx)), None);
}

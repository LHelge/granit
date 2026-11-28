//! Headless window tests of the settings view and of applying settings live.

use gpui::{AppContext as _, Entity, Focusable as _, TestAppContext};
use gpui_kit::component::ActiveTheme as _;
use granit_types::{ProviderConfig, ProviderEntry, ThemeChoice};

use super::fields::probe;
use super::SettingsView;
use crate::config_ops;
use crate::host::Host;
use crate::tests::{fixture_cave, App};

/// Change the config as a settings field does, and let effects settle.
fn update(cx: &mut TestAppContext, f: impl FnOnce(&mut granit_types::AppConfig)) {
    cx.update(|cx| config_ops::update_config(cx, f)).unwrap();
    cx.run_until_parked();
}

/// Open the settings with page `page` shown, and draw them.
fn open_settings(app: &App, page: usize, cx: &mut TestAppContext) -> Entity<SettingsView> {
    cx.update_window(app.window, |_, window, cx| {
        app.workspace
            .update(cx, |workspace, cx| workspace.toggle_settings(window, cx))
    })
    .unwrap();
    let view = cx
        .read_entity(&app.workspace, |workspace, _| workspace.settings().cloned())
        .expect("the settings are open");
    view.update(cx, |view, cx| view.show_page(page, cx));
    cx.run_until_parked();
    app.draw(cx);
    view
}

fn saved_config(dir: &tempfile::TempDir) -> String {
    std::fs::read_to_string(dir.path().join(".granit/config.yml")).unwrap()
}

#[gpui::test]
fn a_theme_change_rethemes_the_editor(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_note(Some("a".into()), cx)
    });
    let view = cx.read_entity(&app.workspace, |workspace, _| workspace.note_view().clone());
    let styled = |cx: &mut TestAppContext| {
        cx.read_entity(&view, |view, cx| {
            (
                cx.theme().is_dark(),
                view.highlight_theme_name(),
                cx.theme().highlight_theme.name.clone(),
            )
        })
    };

    update(cx, |config| config.set_theme_choice(ThemeChoice::Dark));
    let (dark, dark_editor, dark_theme) = styled(cx);
    assert!(dark, "the app is dark");
    assert_eq!(dark_editor, dark_theme, "the editor follows");

    update(cx, |config| config.set_theme_choice(ThemeChoice::Light));
    let (dark, light_editor, light_theme) = styled(cx);
    assert!(!dark, "the app is light");
    assert_eq!(light_editor, light_theme, "the editor follows");
    assert_ne!(light_editor, dark_editor);
    assert!(saved_config(&dir).contains("theme: light"));
    app.draw(cx);
}

#[gpui::test]
fn a_daisyui_theme_in_the_cave_opens_as_its_catppuccin_flavour(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    std::fs::create_dir_all(dir.path().join(".granit")).unwrap();
    std::fs::write(dir.path().join(".granit/config.yml"), "theme: dracula\n").unwrap();

    let app = App::with_cave(dir.path(), cx);

    assert!(cx.update(|cx| cx.theme().is_dark()), "dracula is dark");
    assert_eq!(
        saved_config(&dir),
        "theme: dracula\n",
        "reading the theme rewrites nothing"
    );

    // Opening the settings and drawing them changes nothing either.
    open_settings(&app, 0, cx);
    assert_eq!(saved_config(&dir), "theme: dracula\n");
}

#[gpui::test]
fn the_settings_key_opens_and_closes_the_settings(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    let is_open = |cx: &mut TestAppContext| {
        cx.read_entity(&app.workspace, |workspace, _| {
            workspace.settings().is_some()
        })
    };

    cx.simulate_keystrokes(app.window, crate::workspace::TOGGLE_SETTINGS_KEY);
    cx.run_until_parked();
    assert!(is_open(cx), "the key opens the settings");
    app.draw(cx);

    cx.simulate_keystrokes(app.window, crate::workspace::TOGGLE_SETTINGS_KEY);
    cx.run_until_parked();
    assert!(!is_open(cx), "the key closes them again");

    // Opening a note from the explorer closes them too.
    open_settings(&app, 0, cx);
    app.update_explorer(cx, |explorer, _, cx| explorer.select_row(2, cx));
    assert!(!is_open(cx));
    assert_eq!(app.active_note(cx).as_deref(), Some("a"));
}

#[gpui::test]
fn every_page_draws(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    for page in 0..5 {
        open_settings(&app, page, cx);
        cx.update_window(app.window, |_, window, cx| {
            app.workspace
                .update(cx, |workspace, cx| workspace.toggle_settings(window, cx))
        })
        .unwrap();
        cx.run_until_parked();
        app.draw(cx);
    }
}

#[gpui::test]
fn api_keys_are_masked_and_saved_without_active_cave(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    update(cx, |config| {
        config.agent.providers.push(ProviderEntry {
            name: None,
            provider: ProviderConfig::Anthropic {
                api_key: "sk-old".into(),
            },
        })
    });

    // The Providers page.
    open_settings(&app, 3, cx);

    let key = probe::input("provider-1-api-key").expect("the API key field is drawn");
    assert!(cx.read_entity(&key, |input, _| input.presentation().is_masked()));
    assert_eq!(cx.read_entity(&key, |input, _| input.value()), "sk-old");
    let name = probe::input("provider-1-name").expect("the name field is drawn");
    assert!(!cx.read_entity(&name, |input, _| input.presentation().is_masked()));

    // Typing a key saves it to the cave once typing pauses.
    type_into(&app, &key, &["s", "sk", "sk-new"], cx);
    let provider = cx.update(|cx| Host::core(cx).lock_config().agent.providers[1].clone());
    assert_eq!(provider.provider.api_key(), Some("sk-new"));
    assert!(
        saved_config(&dir).contains("api_key: sk-old"),
        "not saved yet"
    );

    pause(cx);
    let yaml = saved_config(&dir);
    assert!(yaml.contains("api_key: sk-new"), "{yaml}");
    assert!(!yaml.contains("active_cave"), "{yaml}");
    app.draw(cx);
}

/// Replace the text of `input` with each of `texts` in turn, as typing does.
fn type_into(
    app: &App,
    input: &Entity<gpui_kit::component::input::InputState>,
    texts: &[&str],
    cx: &mut TestAppContext,
) {
    for text in texts {
        cx.update_window(app.window, |_, window, cx| {
            input.update(cx, |input, cx| input.replace_all(*text, window, cx))
        })
        .unwrap();
        cx.run_until_parked();
    }
}

/// Let the typing pause long enough for deferred edits to commit.
fn pause(cx: &mut TestAppContext) {
    cx.executor().advance_clock(config_ops::COMMIT_DELAY * 2);
    cx.run_until_parked();
}

fn agent_generation(cx: &mut TestAppContext) -> u64 {
    cx.update(|cx| Host::core(cx).agent_generation())
}

#[gpui::test]
fn typing_in_an_agent_field_resets_the_agent_once(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    open_settings(&app, 3, cx);
    let name = probe::input("provider-0-name").expect("the name field is drawn");
    let generation = agent_generation(cx);

    type_into(&app, &name, &["L", "Lo", "Loc", "Local"], cx);
    assert_eq!(agent_generation(cx), generation, "no reset while typing");

    pause(cx);
    assert_eq!(
        agent_generation(cx),
        generation + 1,
        "one reset for the edit"
    );
    assert!(saved_config(&dir).contains("name: Local"));

    // A switch still applies at once.
    update(cx, |config| config.agent.rag.enabled = false);
    assert_eq!(agent_generation(cx), generation + 2);
}

#[gpui::test]
fn closing_the_settings_commits_a_pending_edit(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    open_settings(&app, 3, cx);
    let name = probe::input("provider-0-name").expect("the name field is drawn");
    type_into(&app, &name, &["Box"], cx);

    cx.update_window(app.window, |_, window, cx| {
        app.workspace
            .update(cx, |workspace, cx| workspace.toggle_settings(window, cx))
    })
    .unwrap();
    cx.run_until_parked();

    assert!(saved_config(&dir).contains("name: Box"));
}

/// Makes a directory of the cave read-only while alive, so the files in it
/// cannot be written (they are replaced atomically, through a file beside
/// them): `.granit` for `config.yml`, `.granit/agent` for the system prompt.
#[cfg(unix)]
struct ReadOnlyConfig(std::path::PathBuf);

#[cfg(unix)]
impl ReadOnlyConfig {
    fn new(dir: &tempfile::TempDir) -> Self {
        Self::dir(dir, ".granit")
    }

    fn dir(dir: &tempfile::TempDir, relative: &str) -> Self {
        let path = dir.path().join(relative);
        Self::set_mode(&path, 0o555);
        Self(path)
    }

    fn set_mode(path: &std::path::Path, mode: u32) {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    }
}

#[cfg(unix)]
impl Drop for ReadOnlyConfig {
    fn drop(&mut self) {
        Self::set_mode(&self.0, 0o755);
    }
}

#[cfg(unix)]
#[gpui::test]
fn a_failed_save_refuses_a_cave_switch_and_keeps_the_edit(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let other = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    let view = open_settings(&app, 3, cx);
    let name = probe::input("provider-0-name").expect("the name field is drawn");
    let generation = agent_generation(cx);

    let read_only = ReadOnlyConfig::new(&dir);
    type_into(&app, &name, &["Box"], cx);
    pause(cx);
    assert!(
        cx.read_entity(&view, |view, _| view.error().is_some()),
        "the failed save is shown"
    );
    assert!(!saved_config(&dir).contains("name: Box"));
    assert_eq!(agent_generation(cx), generation, "nothing was applied");
    assert!(
        cx.update(|cx| config_ops::has_unsaved(cx)),
        "the edit is kept"
    );

    // Another cave does not open while the edit cannot be saved.
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_cave(other.path().to_path_buf(), cx)
    });
    assert_eq!(
        cx.update(|cx| Host::cave_path(cx)),
        Some(dir.path().to_path_buf())
    );
    let failure = cx.read_entity(&app.workspace, |workspace, _| workspace.failure());
    assert!(
        failure.is_some_and(|message| message.contains("settings could not be saved")),
        "the refusal says why"
    );
    assert!(
        cx.read_entity(&app.workspace, |workspace, _| workspace
            .settings()
            .is_some()),
        "the settings stay open"
    );

    // Closing the settings keeps the edit pending too: the switch still refuses.
    cx.update_window(app.window, |_, window, cx| {
        app.workspace
            .update(cx, |workspace, cx| workspace.toggle_settings(window, cx))
    })
    .unwrap();
    cx.run_until_parked();
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_cave(other.path().to_path_buf(), cx)
    });
    assert_eq!(
        cx.update(|cx| Host::cave_path(cx)),
        Some(dir.path().to_path_buf())
    );

    // Once the config can be written, the next save writes the edit and,
    // as it changed the agent config, resets the agent.
    drop(read_only);
    update(cx, |config| config.daily_note_folder = "Journal".into());
    let yaml = saved_config(&dir);
    assert!(yaml.contains("name: Box"), "{yaml}");
    assert!(yaml.contains("Journal"), "{yaml}");
    assert_eq!(agent_generation(cx), generation + 1);

    app.update_workspace(cx, |workspace, cx| {
        workspace.open_cave(other.path().to_path_buf(), cx)
    });
    assert_eq!(
        cx.update(|cx| Host::cave_path(cx)),
        Some(other.path().to_path_buf())
    );
}

#[cfg(unix)]
#[gpui::test]
fn a_system_prompt_that_cannot_be_written_outlives_the_settings(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let other = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    let view = open_settings(&app, 2, cx);
    let generation = agent_generation(cx);
    let prompt_path = dir.path().join(".granit/agent/system.md");
    let original = std::fs::read_to_string(&prompt_path).unwrap();

    let read_only = ReadOnlyConfig::dir(&dir, ".granit/agent");
    let prompt = cx.read_entity(&view, |view, _| view.prompt().clone());
    cx.update_window(app.window, |_, window, cx| {
        prompt.update(cx, |prompt, cx| prompt.replace_all("Be brief.", window, cx))
    })
    .unwrap();
    cx.run_until_parked();

    // Closed before the typing pause: the write fails, and the view is gone.
    cx.update_window(app.window, |_, window, cx| {
        app.workspace
            .update(cx, |workspace, cx| workspace.toggle_settings(window, cx))
    })
    .unwrap();
    cx.run_until_parked();
    assert!(cx.read_entity(&app.workspace, |workspace, _| workspace
        .settings()
        .is_none()));
    assert_eq!(std::fs::read_to_string(&prompt_path).unwrap(), original);
    assert!(
        cx.update(|cx| config_ops::has_unsaved(cx)),
        "the prompt is kept"
    );

    // Another cave does not open while it cannot be written.
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_cave(other.path().to_path_buf(), cx)
    });
    assert_eq!(
        cx.update(|cx| Host::cave_path(cx)),
        Some(dir.path().to_path_buf())
    );
    let failure = cx.read_entity(&app.workspace, |workspace, _| workspace.failure());
    assert!(failure.is_some_and(|message| message.contains("settings could not be saved")));
    assert_eq!(agent_generation(cx), generation);

    // Reopened settings show the pending prompt, not the file, and say it
    // is not saved; closing them untouched keeps it pending.
    let shown_prompt = |cx: &mut TestAppContext| {
        let view = open_settings(&app, 2, cx);
        let prompt = cx.read_entity(&view, |view, _| view.prompt().clone());
        let text = cx.read_entity(&prompt, |prompt, _| prompt.value().to_string());
        let error = cx.read_entity(&view, |view, _| view.error().is_some());
        (prompt, text, error)
    };
    let close_settings = |cx: &mut TestAppContext| {
        cx.update_window(app.window, |_, window, cx| {
            app.workspace
                .update(cx, |workspace, cx| workspace.toggle_settings(window, cx))
        })
        .unwrap();
        cx.run_until_parked();
    };
    let (_, text, error) = shown_prompt(cx);
    assert_eq!(text, "Be brief.");
    assert!(error, "the failed write is shown again");
    close_settings(cx);
    assert!(cx.update(|cx| config_ops::has_unsaved(cx)));

    // Editing the shown prompt replaces the pending one.
    let (prompt, text, _) = shown_prompt(cx);
    assert_eq!(text, "Be brief.", "still pending after an untouched close");
    cx.update_window(app.window, |_, window, cx| {
        prompt.update(cx, |prompt, cx| {
            prompt.replace_all("Be briefer.", window, cx)
        })
    })
    .unwrap();
    cx.run_until_parked();
    close_settings(cx);
    assert_eq!(std::fs::read_to_string(&prompt_path).unwrap(), original);

    // Once it can be, the switch writes it (resetting the agent) and opens.
    drop(read_only);
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_cave(other.path().to_path_buf(), cx)
    });
    assert_eq!(
        std::fs::read_to_string(&prompt_path).unwrap(),
        "Be briefer."
    );
    assert_eq!(
        cx.update(|cx| Host::cave_path(cx)),
        Some(other.path().to_path_buf())
    );
    assert!(!cx.update(|cx| config_ops::has_unsaved(cx)));
}

#[cfg(unix)]
#[gpui::test]
fn a_successful_write_keeps_a_pending_failure_shown(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    let view = open_settings(&app, 3, cx);
    let name = probe::input("provider-0-name").expect("the name field is drawn");

    let _read_only = ReadOnlyConfig::new(&dir);
    type_into(&app, &name, &["Box"], cx);
    pause(cx);
    assert!(cx.read_entity(&view, |view, _| view.error().is_some()));

    // The system prompt lives in `.granit/agent/`, which is still writable.
    let prompt = cx.read_entity(&view, |view, _| view.prompt().clone());
    cx.update_window(app.window, |_, window, cx| {
        prompt.update(cx, |prompt, cx| prompt.replace_all("Be brief.", window, cx))
    })
    .unwrap();
    pause(cx);
    assert_eq!(
        std::fs::read_to_string(dir.path().join(".granit/agent/system.md")).unwrap(),
        "Be brief."
    );
    assert!(
        cx.read_entity(&view, |view, _| view.error().is_some()),
        "the unsaved name is still reported"
    );
}

#[cfg(unix)]
#[gpui::test]
fn a_successful_deferred_save_clears_a_stale_error(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    let view = open_settings(&app, 3, cx);
    let name = probe::input("provider-0-name").expect("the name field is drawn");

    let read_only = ReadOnlyConfig::new(&dir);
    type_into(&app, &name, &["Box"], cx);
    pause(cx);
    assert!(cx.read_entity(&view, |view, _| view.error().is_some()));

    // Writable again: the next typing pause saves, and the error goes.
    drop(read_only);
    type_into(&app, &name, &["Boxes"], cx);
    pause(cx);
    assert!(saved_config(&dir).contains("name: Boxes"));
    assert!(
        cx.read_entity(&view, |view, _| view.error().is_none()),
        "the stale error is cleared"
    );
}

#[gpui::test]
fn number_fields_commit_on_enter_and_blur(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    open_settings(&app, 2, cx);
    let field = probe::input("max-history").expect("the max history field is drawn");
    let generation = agent_generation(cx);

    // Focus events reach only an active window.
    cx.update_window(app.window, |_, window, cx| {
        window.activate_window();
        field.update(cx, |input, cx| input.focus(window, cx))
    })
    .unwrap();
    app.draw(cx);
    type_into(&app, &field, &["4", "42"], cx);
    assert_eq!(
        cx.update(|cx| Host::core(cx).lock_config().agent.max_history),
        42
    );
    assert!(
        !saved_config(&dir).contains("max_history: 42"),
        "not saved yet"
    );

    // Enter commits at once, without the typing pause.
    cx.simulate_keystrokes(app.window, "enter");
    cx.run_until_parked();
    assert!(saved_config(&dir).contains("max_history: 42"));
    assert_eq!(agent_generation(cx), generation + 1);

    // So does leaving the field.
    type_into(&app, &field, &["43"], cx);
    cx.update_window(app.window, |_, window, cx| {
        app.workspace.update(cx, |workspace, cx| {
            window.focus(&workspace.focus_handle(cx), cx)
        })
    })
    .unwrap();
    app.draw(cx);
    cx.run_until_parked();
    assert!(saved_config(&dir).contains("max_history: 43"));
    assert_eq!(agent_generation(cx), generation + 2);
}

#[gpui::test]
fn editing_the_system_prompt_writes_it_once_and_resets_the_agent(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    let view = open_settings(&app, 2, cx);
    let generation = agent_generation(cx);

    let prompt = cx.read_entity(&view, |view, _| view.prompt().clone());
    for text in ["B", "Be", "Be brief."] {
        cx.update_window(app.window, |_, window, cx| {
            prompt.update(cx, |prompt, cx| prompt.replace_all(text, window, cx))
        })
        .unwrap();
        cx.run_until_parked();
    }
    assert_eq!(agent_generation(cx), generation, "no reset while typing");

    pause(cx);
    let written = std::fs::read_to_string(dir.path().join(".granit/agent/system.md")).unwrap();
    assert_eq!(written, "Be brief.");
    assert_eq!(agent_generation(cx), generation + 1);
    assert!(cx.read_entity(&view, |view, _| view.error().is_none()));
}

#[gpui::test]
fn font_changes_apply_to_the_editor_and_chat_live(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_note(Some("a".into()), cx);
        workspace.toggle_chat(cx);
    });
    app.draw(cx);
    let (note_view, chat) = cx.read_entity(&app.workspace, |workspace, _| {
        (workspace.note_view().clone(), workspace.chat().clone())
    });
    let drawn = |cx: &mut TestAppContext| {
        (
            cx.read_entity(&note_view, |view, _| view.drawn_font()),
            cx.read_entity(&chat, |chat, _| chat.drawn_font()),
        )
    };

    // Defaults: the note in the theme's UI font at 16, the chat at 14.
    let ui_font = cx.update(|cx| cx.theme().font_family.clone());
    let (note, chat_font) = drawn(cx);
    assert_eq!(note, Some((ui_font.clone(), gpui::px(16.))));
    assert_eq!(chat_font, Some((ui_font, gpui::px(14.))));

    update(cx, |config| {
        config.reading_font.font_family = "monospace".into();
        config.reading_font.font_size = 20;
        config.agent_font.font_size = 11;
    });
    app.draw(cx);

    let mono = cx.update(|cx| cx.theme().mono_font_family.clone());
    let (note, chat_font) = drawn(cx);
    assert_eq!(note, Some((mono, gpui::px(20.))));
    assert_eq!(chat_font.map(|(_, size)| size), Some(gpui::px(11.)));
}

#[test]
fn the_font_list_keeps_the_generics_and_an_unknown_stored_family() {
    use super::fields::font_choices;
    use gpui_kit::component::searchable_list::SearchableListItem as _;

    let theme = gpui_kit::component::Theme::default();
    let fonts: Vec<gpui::SharedString> = vec!["Fira Sans".into(), "Iosevka".into()];
    let values = |current: &str| -> Vec<String> {
        font_choices(&fonts, current, &theme)
            .iter()
            .map(|choice| choice.value().to_string())
            .collect()
    };
    assert_eq!(
        values("Iosevka"),
        ["sans-serif", "monospace", "Fira Sans", "Iosevka"]
    );
    assert_eq!(
        values("sans-serif"),
        ["sans-serif", "monospace", "Fira Sans", "Iosevka"]
    );
    // Another generic, or a font no longer installed, stays selectable.
    assert_eq!(
        values("serif"),
        ["sans-serif", "monospace", "serif", "Fira Sans", "Iosevka"]
    );

    let choices = font_choices(&fonts, "", &theme);
    let matching = |query: &str| -> Vec<String> {
        choices
            .iter()
            .filter(|choice| choice.matches(query))
            .map(|choice| choice.title().to_string())
            .collect()
    };
    assert_eq!(matching("iose"), ["Iosevka"]);
    assert_eq!(matching("MONO"), ["Theme monospace font"]);
    assert_eq!(matching("sans"), ["Theme font", "Fira Sans"]);
}

#[gpui::test]
fn picking_a_font_saves_it_and_follows_the_config(cx: &mut TestAppContext) {
    use gpui_kit::component::select::SelectEvent;

    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    open_settings(&app, 0, cx);
    let select = probe::font("font-family-Note font").expect("the note font picker is drawn");
    let selected = |cx: &mut TestAppContext| {
        cx.read_entity(&select, |select, _| select.selected_value().cloned())
    };
    assert_eq!(selected(cx).as_deref(), Some("sans-serif"));

    // A pick is saved at once.
    select.update(cx, |_, cx| {
        cx.emit(SelectEvent::Confirm(Some("monospace".into())))
    });
    cx.run_until_parked();
    let family = |cx: &mut TestAppContext| {
        cx.update(|cx| {
            Host::core(cx)
                .lock_config()
                .reading_font
                .font_family
                .clone()
        })
    };
    assert_eq!(family(cx), "monospace");
    assert!(saved_config(&dir).contains("monospace"));

    // A change made elsewhere shows as the choice, even a family the list
    // did not have.
    update(cx, |config| {
        config.reading_font.font_family = "serif".into()
    });
    app.draw(cx);
    assert_eq!(selected(cx).as_deref(), Some("serif"));
}

#[gpui::test]
fn a_chat_pick_commits_a_pending_field_edit_first(cx: &mut TestAppContext) {
    use std::cell::Cell;
    use std::rc::Rc;

    let dir = fixture_cave();
    let _app = App::with_cave(dir.path(), cx);
    let agent_events = Rc::new(Cell::new(0));
    let _subscription = cx.update(|cx| {
        let agent_events = agent_events.clone();
        cx.subscribe(
            &config_ops::ConfigEvents::entity(cx),
            move |_, event: &config_ops::ConfigEvent, _| {
                if event.agent {
                    agent_events.set(agent_events.get() + 1);
                }
            },
        )
    });
    let generation = agent_generation(cx);

    // A Brave key typed in the settings, waiting for its typing pause.
    cx.update(|cx| {
        config_ops::edit_config(cx, |config| {
            config.agent.tool_config.web_search.api_key = Some("BSA-key".into())
        })
    });
    cx.run_until_parked();

    // The model is picked in the chat meanwhile.
    let changed = cx
        .update(|cx| crate::agent_ops::select_model("qwen3".into(), cx))
        .unwrap();
    cx.run_until_parked();
    assert!(changed);

    // The edit was committed and announced first, then the pick saved.
    let yaml = saved_config(&dir);
    assert!(yaml.contains("api_key: BSA-key"), "{yaml}");
    assert!(yaml.contains("selected_model: qwen3"), "{yaml}");
    assert_eq!(agent_events.get(), 1, "the settings edit is announced");
    assert_eq!(agent_generation(cx), generation + 2, "one reset each");

    // Nothing is left to commit after the pause.
    pause(cx);
    assert_eq!(agent_generation(cx), generation + 2);
    assert_eq!(agent_events.get(), 1);

    // Re-picking the same model changes nothing.
    let changed = cx
        .update(|cx| crate::agent_ops::select_model("qwen3".into(), cx))
        .unwrap();
    assert!(!changed);
    assert_eq!(agent_generation(cx), generation + 2);
}

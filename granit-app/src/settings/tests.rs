//! Headless window tests of the settings view and of applying settings live.

use gpui::{AppContext as _, Entity, TestAppContext};
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
    for page in 0..4 {
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

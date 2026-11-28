//! Headless tests of the chat panel: a canned agent stream (no network)
//! rendered into the view, its links, and the panel's controls.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    point, px, AppContext as _, Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext,
};
use gpui_kit::base::text::Link;
use granit_core::agent::{AgentError, AgentStream, AgentStreamItem};
use granit_core::cave::Cave;
use granit_types::{AgentMode, AppConfig, ToolCallInfo};

use super::{selection_preview, ChatEvent, ChatPanel};
use crate::config_ops::{ConfigEvent, ConfigEvents};
use crate::events::CaveEvents;
use crate::host::Host;
use crate::tests::{fixture_cave, App};

/// Open the fixture cave (plus `cars.md` with a `{#volvo}` anchor) with
/// the chat shown. No provider is configured, so nothing is asked over the
/// network.
fn start(cx: &mut TestAppContext) -> (tempfile::TempDir, App, Entity<ChatPanel>) {
    let dir = fixture_cave();
    std::fs::write(dir.path().join("cars.md"), "# Volvo {#volvo}\n").unwrap();
    let app = App::with_cave(dir.path(), cx);
    cx.update(|cx| Host::core(cx).lock_config().agent.providers.clear());
    app.update_workspace(cx, |workspace, cx| workspace.toggle_chat(cx));
    let chat = cx.read_entity(&app.workspace, |workspace, _| workspace.chat().clone());
    (dir, app, chat)
}

/// Paint twice with time to parse in between: the first frame hands the
/// view its Markdown extensions, which reparses the text with them.
fn settle(app: &App, cx: &mut TestAppContext) {
    for _ in 0..2 {
        cx.run_until_parked();
        app.draw(cx);
    }
    cx.run_until_parked();
}

fn text(chunk: &str) -> Result<AgentStreamItem, AgentError> {
    Ok(AgentStreamItem::Text(chunk.to_string()))
}

/// The config saved in the cave at `dir`.
fn saved_config(dir: &tempfile::TempDir) -> AppConfig {
    Cave::open(dir.path().to_path_buf())
        .unwrap()
        .load_config()
        .unwrap()
}

#[gpui::test]
async fn a_streamed_reply_renders_with_links_tools_and_code(cx: &mut TestAppContext) {
    let (_dir, app, chat) = start(cx);
    let events = Rc::new(Cell::new(0));
    let _counting = cx.update(|cx| {
        let events = events.clone();
        cx.subscribe(&CaveEvents::entity(cx), move |_, _, _| {
            events.set(events.get() + 1)
        })
    });

    let items = vec![
        text("Here is [["),
        text("a]] and [[Volvo|the car]] and [[ghost]].\n\n```\nlet x = \"[["),
        text("code]]\";\n```\n\n- [ ] a task\n"),
        Ok(AgentStreamItem::ToolCall(ToolCallInfo {
            name: "read_note".into(),
            param: Some("a".into()),
        })),
        Ok(AgentStreamItem::ToolResult),
        text("Done, see `[[b]]`."),
        Ok(AgentStreamItem::Done),
    ];
    let task = chat.update(cx, |chat, cx| {
        chat.run_stream(
            async move { Ok(AgentStream::from_items(items)) },
            "hi".into(),
            0,
            cx,
        );
        assert!(chat.is_streaming());
        chat.take_stream().unwrap()
    });
    // The stream is read on the tokio runtime, outside GPUI's test
    // executor.
    cx.executor().allow_parking();
    task.await;
    settle(&app, cx);

    let source = cx.read_entity(&chat, |chat, _| chat.transcript().to_string());
    assert!(
        source.starts_with(
            "Here is [a](<wiki:a>) and [the car](<wiki:Volvo>) and [ghost](<wiki:ghost>)."
        ),
        "{source}"
    );
    assert!(source.contains("let x = \"[[code]]\";"), "{source}");
    assert!(
        source.contains("> [!tool]\n> `read_note(a)`\n\nDone, see `[[b]]`."),
        "{source}"
    );

    let rendered = cx.read_entity(&chat, |chat, cx| chat.rendered_text(cx));
    for expected in [
        "Here is a and the car and ghost.",
        "let x = \"[[code]]\";",
        "a task",
        "read_note(a)",
        "Done, see [[b]].",
    ] {
        assert!(rendered.contains(expected), "{expected:?} in {rendered:?}");
    }
    assert!(
        !rendered.contains("[!tool]"),
        "the surface marker is not text: {rendered:?}"
    );

    // The view parsed exactly the rewritten links, none from code.
    let parsed = cx.read_entity(&chat, |chat, _| chat.parsed_links());
    assert_eq!(parsed, vec!["wiki:Volvo", "wiki:a", "wiki:ghost"]);
    // Resolved links point at their note (an anchor at its note), a
    // broken one is drawn as text.
    let targets = cx.read_entity(&chat, |chat, _| {
        ["wiki:a", "wiki:Volvo", "wiki:ghost"].map(|url| {
            chat.link_targets()
                .retarget(&Link::new(url))
                .map(|link| link.url.to_string())
        })
    });
    assert_eq!(
        targets,
        [
            Some("wiki:a".to_string()),
            Some("wiki:cars#volvo".to_string()),
            None
        ]
    );

    // The tool result and the end of the reply both refresh the views.
    assert_eq!(events.get(), 2);
    assert!(!cx.read_entity(&chat, |chat, _| chat.is_streaming()));
}

#[gpui::test]
fn a_clicked_wiki_link_opens_its_note(cx: &mut TestAppContext) {
    let (_dir, app, chat) = start(cx);
    chat.update(cx, |_, cx| {
        cx.emit(ChatEvent::OpenNote {
            slug: "cars".into(),
            anchor: Some("volvo".into()),
        })
    });
    cx.run_until_parked();
    assert_eq!(app.active_note(cx).as_deref(), Some("cars"));
}

#[gpui::test]
fn changing_the_mode_mid_stream_cancels_the_reply_and_resets_the_agent(cx: &mut TestAppContext) {
    let (dir, app, chat) = start(cx);
    chat.update(cx, |chat, cx| {
        chat.run_stream(std::future::pending(), "hi".into(), 0, cx);
        chat.on_reply_text("Partial", cx);
    });
    let generation = cx.update(|cx| Host::core(cx).agent_generation());

    chat.update(cx, |chat, cx| chat.select_mode(AgentMode::Ask, cx));
    cx.run_until_parked();

    assert!(!cx.read_entity(&chat, |chat, _| chat.is_streaming()));
    assert_eq!(
        cx.read_entity(&chat, |chat, _| chat.transcript().to_string()),
        ""
    );
    cx.update(|cx| {
        let core = Host::core(cx);
        assert_eq!(core.lock_config().agent.mode, AgentMode::Ask);
        assert!(core.agent_generation() > generation, "the agent was reset");
    });
    assert_eq!(
        saved_config(&dir).agent.mode,
        AgentMode::Ask,
        "the mode is saved"
    );
    app.draw(cx);
}

#[gpui::test]
fn sending_without_a_provider_shows_the_error(cx: &mut TestAppContext) {
    let (_dir, app, chat) = start(cx);
    chat.update(cx, |chat, cx| chat.submit("Hello [[a]]".into(), cx));
    settle(&app, cx);

    let source = cx.read_entity(&chat, |chat, _| chat.transcript().to_string());
    assert!(
        source.starts_with("> [!user]\n> Hello [a](<wiki:a>)\n\n> [!error]\n"),
        "{source}"
    );
    assert!(!cx.read_entity(&chat, |chat, _| chat.is_streaming()));

    // A new conversation starts empty.
    chat.update(cx, |chat, cx| chat.new_conversation(cx));
    assert_eq!(
        cx.read_entity(&chat, |chat, _| chat.transcript().to_string()),
        ""
    );
}

#[gpui::test]
fn the_chat_toggle_is_remembered_in_the_cave(cx: &mut TestAppContext) {
    let (dir, app, _chat) = start(cx);
    assert!(cx.read_entity(&app.workspace, |workspace, _| workspace.chat_visible()));
    assert!(saved_config(&dir).agent_panel.visible);

    cx.simulate_keystrokes(app.window, crate::workspace::TOGGLE_CHAT_KEY);
    cx.run_until_parked();
    assert!(!cx.read_entity(&app.workspace, |workspace, _| workspace.chat_visible()));
    assert!(!saved_config(&dir).agent_panel.visible);
}

#[gpui::test]
fn re_picking_the_current_mode_keeps_the_conversation(cx: &mut TestAppContext) {
    let (_dir, _app, chat) = start(cx);
    let mode = cx.update(|cx| Host::core(cx).lock_config().agent.mode);
    chat.update(cx, |chat, cx| {
        chat.run_stream(std::future::pending(), "hi".into(), 0, cx);
        chat.on_reply_text("Partial", cx);
    });
    let generation = cx.update(|cx| Host::core(cx).agent_generation());

    chat.update(cx, |chat, cx| chat.select_mode(mode, cx));
    cx.run_until_parked();

    assert!(cx.read_entity(&chat, |chat, _| chat.is_streaming()));
    assert_eq!(
        cx.read_entity(&chat, |chat, _| chat.transcript().to_string()),
        "Partial"
    );
    assert_eq!(
        cx.update(|cx| Host::core(cx).agent_generation()),
        generation,
        "the agent is kept"
    );
}

#[gpui::test]
fn the_open_note_and_its_selection_reach_the_prompt(cx: &mut TestAppContext) {
    let (_dir, app, chat) = start(cx);
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_note(Some("a".into()), cx)
    });
    app.draw(cx);
    // The open note is offered, nothing is attached yet.
    assert!(cx.read_entity(&chat, |chat, _| chat.attached().is_empty()));
    chat.update(cx, |chat, cx| chat.submit("Plain".into(), cx));
    assert_eq!(
        cx.read_entity(&chat, |chat, _| chat.last_prompt().map(str::to_string)),
        Some("Plain".to_string()),
        "the open note is not attached by default"
    );

    // Select from "Alpha" to the end of its line in the editor.
    let note_view = cx.read_entity(&app.workspace, |workspace, _| workspace.note_view().clone());
    let editor = cx
        .read_entity(&note_view, |view, _| view.editor().cloned())
        .expect("the editor is installed");
    let body = "# A\n\nAlpha links [[b]].\n";
    cx.update_window(app.window, |_, window, cx| {
        editor.update(cx, |editor, cx| {
            editor.set_cursor(body.find("Alpha").unwrap(), cx);
            editor.focus(window, cx);
        })
    })
    .unwrap();
    app.draw(cx);
    cx.simulate_keystrokes(app.window, "shift-end");
    cx.run_until_parked();

    // The attach button asks the workspace, which reads the editor.
    chat.update(cx, |_, cx| cx.emit(ChatEvent::AttachActiveNote));
    cx.run_until_parked();
    app.draw(cx);
    let attached = cx.read_entity(&chat, |chat, _| chat.attached().to_vec());
    assert_eq!(attached.len(), 1);
    assert_eq!(attached[0].slug, "a");
    assert_eq!(attached[0].content, body);
    assert_eq!(attached[0].selected.as_deref(), Some("Alpha links [[b]]."));

    // Attaching it again keeps the one attachment.
    chat.update(cx, |_, cx| cx.emit(ChatEvent::AttachActiveNote));
    cx.run_until_parked();
    assert_eq!(cx.read_entity(&chat, |chat, _| chat.attached().len()), 1);

    chat.update(cx, |chat, cx| chat.submit("Summarize it".into(), cx));
    let prompt = cx
        .read_entity(&chat, |chat, _| chat.last_prompt().map(str::to_string))
        .unwrap();
    for expected in [
        "<attached_note>\n<slug>a</slug>\n",
        "<selected_text>\nAlpha links [[b]].\n</selected_text>\n",
        "<content>\n# A\n\nAlpha links [[b]].\n\n</content>\n",
        "<user_request>\nSummarize it\n</user_request>",
    ] {
        assert!(prompt.contains(expected), "{expected:?} in {prompt:?}");
    }
    // The chat shows what was typed, not the attachments.
    let source = cx.read_entity(&chat, |chat, _| chat.transcript().to_string());
    assert!(!source.contains("attached_note"), "{source}");

    // A removed attachment is no longer sent.
    chat.update(cx, |chat, cx| chat.detach("a", cx));
    chat.update(cx, |chat, cx| chat.submit("Again".into(), cx));
    assert_eq!(
        cx.read_entity(&chat, |chat, _| chat.last_prompt().map(str::to_string)),
        Some("Again".to_string())
    );
    app.draw(cx);
}

#[gpui::test]
fn a_template_is_not_offered_for_attaching(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    std::fs::create_dir_all(dir.path().join(".granit/templates")).unwrap();
    std::fs::write(
        dir.path().join(".granit/templates/meeting.md"),
        "# Meeting\n",
    )
    .unwrap();
    let app = App::with_cave(dir.path(), cx);
    let chat = cx.read_entity(&app.workspace, |workspace, _| workspace.chat().clone());
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_note(Some("a".into()), cx);
        workspace.open_template("meeting".into(), cx)
    });
    assert_eq!(
        cx.read_entity(&app.workspace, |workspace, cx| workspace
            .open_template_slug(cx)),
        Some("meeting".to_string())
    );
    chat.update(cx, |_, cx| cx.emit(ChatEvent::AttachActiveNote));
    cx.run_until_parked();
    assert!(cx.read_entity(&chat, |chat, _| chat.attached().is_empty()));
}

#[test]
fn a_long_selection_is_shortened_to_one_line() {
    assert_eq!(selection_preview("  two\n words "), "two words");
    assert_eq!(
        selection_preview("a selection that is longer than the chip"),
        "a selection that is longer t…"
    );
}

#[gpui::test]
fn dragging_the_edge_resizes_the_panel_and_saves_the_width_once(cx: &mut TestAppContext) {
    let (dir, app, chat) = start(cx);
    assert_eq!(saved_config(&dir).agent_panel.width, 320);
    app.draw(cx);
    let mut window = VisualTestContext::from_window(app.window, cx);
    let viewport = window.update(|window, _| window.viewport_size());
    let edge = viewport.width - px(320.);
    let y = viewport.height / 2.;
    let saves = Rc::new(Cell::new(0));
    let _counting = window.update(|_, cx| {
        let saves = saves.clone();
        cx.subscribe(&ConfigEvents::entity(cx), move |_, _: &ConfigEvent, _| {
            saves.set(saves.get() + 1)
        })
    });

    window.simulate_mouse_move(point(edge + px(2.), y), None, Modifiers::default());
    window.simulate_mouse_down(
        point(edge + px(2.), y),
        MouseButton::Left,
        Modifiers::default(),
    );
    for x in [edge - px(20.), edge - px(80.)] {
        window.simulate_mouse_move(point(x, y), MouseButton::Left, Modifiers::default());
        app.draw(&mut window);
    }
    assert!(cx.read_entity(&chat, |chat, _| chat.resize().is_dragging()));
    assert_eq!(saves.get(), 0, "nothing is saved while dragging");
    assert_eq!(saved_config(&dir).agent_panel.width, 320);

    window.simulate_mouse_up(
        point(edge - px(80.), y),
        MouseButton::Left,
        Modifiers::default(),
    );
    app.draw(&mut window);
    assert!(!cx.read_entity(&chat, |chat, _| chat.resize().is_dragging()));
    assert_eq!(saves.get(), 1, "the width is saved once");
    assert_eq!(saved_config(&dir).agent_panel.width, 400);
    assert_eq!(
        cx.update(|cx| Host::core(cx).lock_config().agent_panel.width),
        400
    );
}

/// Stream `items` (ending with `Done`) into the chat and wait for the end.
async fn stream(
    app: &App,
    chat: &Entity<ChatPanel>,
    items: Vec<Result<AgentStreamItem, AgentError>>,
    cx: &mut TestAppContext,
) {
    let task = chat.update(cx, |chat, cx| {
        chat.run_stream(
            async move { Ok(AgentStream::from_items(items)) },
            "hi".into(),
            0,
            cx,
        );
        chat.take_stream().unwrap()
    });
    cx.executor().allow_parking();
    task.await;
    settle(app, cx);
}

#[gpui::test]
async fn indented_code_in_a_list_item_is_code_as_in_the_reader(cx: &mut TestAppContext) {
    let (_dir, app, chat) = start(cx);
    let items = vec![
        text("- item\n\n      [[x"),
        text("]]\n\nAfter [[a]]."),
        Ok(AgentStreamItem::Done),
    ];
    stream(&app, &chat, items, cx).await;

    let source = cx.read_entity(&chat, |chat, _| chat.transcript().to_string());
    assert_eq!(
        source, "- item\n\n      [[x]]\n\nAfter [a](<wiki:a>).",
        "the list's code is left as written"
    );
    let parsed = cx.read_entity(&chat, |chat, _| chat.parsed_links());
    assert_eq!(parsed, vec!["wiki:a"]);
    let rendered = cx.read_entity(&chat, |chat, cx| chat.rendered_text(cx));
    assert!(rendered.contains("[[x]]"), "{rendered:?}");
}

#[gpui::test]
async fn links_and_fences_split_across_chunks_are_decided_whole(cx: &mut TestAppContext) {
    let (_dir, app, chat) = start(cx);
    let items = vec![
        text("Before [[a"),
        text("|the "),
        text("note]] and `[[no"),
        text("t]]` then\n``"),
        text("`\n[[b]]\n`"),
        text("``\nAfter [["),
        text("c]]"),
        Ok(AgentStreamItem::Done),
    ];
    stream(&app, &chat, items, cx).await;

    let source = cx.read_entity(&chat, |chat, _| chat.transcript().to_string());
    assert_eq!(
        source,
        "Before [the note](<wiki:a>) and `[[not]]` then\n```\n[[b]]\n```\nAfter [c](<wiki:c>)"
    );
    let parsed = cx.read_entity(&chat, |chat, _| chat.parsed_links());
    assert_eq!(parsed, vec!["wiki:a", "wiki:c"]);
}

#[gpui::test]
fn a_cave_switch_stops_the_reply_before_the_core_swaps_caves(cx: &mut TestAppContext) {
    let (dir, app, chat) = start(cx);
    chat.update(cx, |chat, cx| {
        chat.run_stream(std::future::pending(), "hi".into(), 0, cx);
        chat.on_reply_text("Partial", cx);
    });
    assert!(cx.read_entity(&chat, |chat, _| chat.is_streaming()));

    let other = fixture_cave();
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_cave(other.path().to_path_buf(), cx)
    });
    cx.run_until_parked();

    assert_eq!(
        cx.update(|cx| Host::cave_path(cx)),
        Some(other.path().to_path_buf())
    );
    assert!(!cx.read_entity(&chat, |chat, _| chat.is_streaming()));
    assert_eq!(
        cx.read_entity(&chat, |chat, _| chat.stopped_in()),
        Some(Some(dir.path().to_path_buf())),
        "the reply stopped while the old cave was still installed"
    );
}

#[gpui::test]
async fn link_labels_render_as_written(cx: &mut TestAppContext) {
    let (_dir, app, chat) = start(cx);
    let items = vec![
        text("See [[*x*]] and [[t|a\\b]], "),
        text("not [[a\\]]."),
        Ok(AgentStreamItem::Done),
    ];
    let task = chat.update(cx, |chat, cx| {
        chat.run_stream(
            async move { Ok(AgentStream::from_items(items)) },
            "hi".into(),
            0,
            cx,
        );
        chat.take_stream().unwrap()
    });
    cx.executor().allow_parking();
    task.await;
    settle(&app, cx);

    let rendered = cx.read_entity(&chat, |chat, cx| chat.rendered_text(cx));
    // `\]` escapes the bracket, so the reader sees no link there either.
    assert!(
        rendered.contains("See *x* and a\\b, not [[a]]."),
        "{rendered:?}"
    );
    let parsed = cx.read_entity(&chat, |chat, _| chat.parsed_links());
    assert_eq!(parsed, vec!["wiki:*x*", "wiki:t"]);
}

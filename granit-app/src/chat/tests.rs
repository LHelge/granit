//! Headless tests of the chat panel: a canned agent stream (no network)
//! rendered into the view, its links, and the panel's controls.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{AppContext as _, Entity, TestAppContext};
use gpui_kit::base::text::Link;
use granit_core::agent::{AgentError, AgentStream, AgentStreamItem};
use granit_core::cave::Cave;
use granit_types::{AgentMode, AppConfig, ToolCallInfo};

use super::{ChatEvent, ChatPanel};
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
async fn link_labels_render_as_written(cx: &mut TestAppContext) {
    let (_dir, app, chat) = start(cx);
    let items = vec![text("See [[*x*]] and [[a\\]]."), Ok(AgentStreamItem::Done)];
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
    assert!(rendered.contains("See *x* and a\\."), "{rendered:?}");
    let parsed = cx.read_entity(&chat, |chat, _| chat.parsed_links());
    assert_eq!(parsed, vec!["wiki:*x*", "wiki:a\\"]);
}

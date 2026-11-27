//! Headless window tests of the note editor: loading and saving, the
//! stale-layout presses, wiki-links and completion, find, images, and the
//! byte-identical round-trip of the edge-case fixtures.

use std::path::Path;

use gpui::{
    point, px, AppContext as _, Context, Entity, EntityInputHandler, Focusable as _, Modifiers,
    MouseButton, MouseDownEvent, MouseUpEvent, Pixels, PlatformInput, Point, TestAppContext,
    VisualTestContext, Window,
};
use granit_core::agent::{AgentStream, AgentStreamItem};
use granit_core::cave::{DocumentHeader, HeaderEdit};
use granit_core::markdown::Markdown;
use zorite_editor::EditorState;

use super::save::{FlushOutcome, SaveStatus};
use super::{wiki_links, NoteView, Reveal, AUTOSAVE_DELAY};
use crate::cave_ops;
use crate::events::{AgentDocKind, CaveEvent, CaveEvents};
use crate::host::Host;
use crate::tests::{fixture_cave, App};

const NOTE_A_BODY: &str = "# A\n\nAlpha links [[b]].\n";
const NOTE_B: &str = "# B\n";

fn view(app: &App, cx: &mut TestAppContext) -> Entity<NoteView> {
    cx.read_entity(&app.workspace, |workspace, _| workspace.note_view().clone())
}

/// Run `f` on the note view with the window, then let effects settle.
fn update_view<R>(
    app: &App,
    cx: &mut TestAppContext,
    f: impl FnOnce(&mut NoteView, &mut Window, &mut Context<NoteView>) -> R,
) -> R {
    let view = view(app, cx);
    let result = cx
        .update_window(app.window, |_, window, cx| {
            view.update(cx, |view, cx| f(view, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
    result
}

fn editor(app: &App, cx: &mut TestAppContext) -> Entity<EditorState> {
    let view = view(app, cx);
    cx.read_entity(&view, |view, _| view.editor().cloned())
        .expect("the editor is installed")
}

/// Run `f` on the editor with the window, then let effects settle.
fn update_editor<R>(
    app: &App,
    cx: &mut TestAppContext,
    f: impl FnOnce(&mut EditorState, &mut Window, &mut Context<EditorState>) -> R,
) -> R {
    let editor = editor(app, cx);
    let result = cx
        .update_window(app.window, |_, window, cx| {
            editor.update(cx, |editor, cx| f(editor, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
    result
}

fn text(app: &App, cx: &mut TestAppContext) -> String {
    let view = view(app, cx);
    cx.read_entity(&view, |view, cx| {
        view.text(cx).unwrap_or_default().to_string()
    })
}

fn cursor(app: &App, cx: &mut TestAppContext) -> usize {
    let editor = editor(app, cx);
    cx.read_entity(&editor, |editor, _| editor.cursor())
}

fn open(app: &App, slug: &str, cx: &mut TestAppContext) {
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_note(Some(slug.to_string()), cx)
    });
}

/// Append `text` at the end of the note as typed input.
fn type_at_end(app: &App, text: &str, cx: &mut TestAppContext) {
    update_editor(app, cx, |editor, window, cx| {
        let end = editor.text().len();
        editor.set_cursor(end, cx);
        editor.replace_text_in_range(None, text, window, cx);
    });
}

/// Put the caret at `offset`, focus the editor and paint, so typed input
/// and bound keys reach it.
fn focus_at(app: &App, offset: usize, cx: &mut TestAppContext) {
    update_editor(app, cx, |editor, window, cx| {
        editor.set_cursor(offset, cx);
        editor.focus(window, cx);
    });
    app.draw(cx);
}

fn focus_at_end(app: &App, cx: &mut TestAppContext) {
    let end = text(app, cx).len();
    focus_at(app, end, cx);
}

fn type_text(app: &App, text: &str, cx: &mut TestAppContext) {
    cx.simulate_input(app.window, text);
    cx.run_until_parked();
    app.draw(cx);
}

fn keys(app: &App, keys: &str, cx: &mut TestAppContext) {
    cx.simulate_keystrokes(app.window, keys);
    cx.run_until_parked();
    app.draw(cx);
}

fn read(dir: &Path, name: &str) -> String {
    std::fs::read_to_string(dir.join(name)).unwrap()
}

fn flush(app: &App, cx: &mut TestAppContext) -> FlushOutcome {
    update_view(app, cx, |view, _, cx| view.flush(cx))
}

#[gpui::test]
fn opening_and_switching_without_edits_writes_nothing(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let note_a = read(dir.path(), "a.md");
    let app = App::with_cave(dir.path(), cx);
    open(&app, "a", cx);
    assert_eq!(
        text(&app, cx),
        NOTE_A_BODY,
        "the editor never sees frontmatter"
    );
    app.draw(cx);
    open(&app, "b", cx);
    assert_eq!(text(&app, cx), NOTE_B);
    app.draw(cx);
    open(&app, "a", cx);
    cx.executor().advance_clock(AUTOSAVE_DELAY * 2);
    cx.run_until_parked();

    assert_eq!(read(dir.path(), "a.md"), note_a);
    assert_eq!(read(dir.path(), "sub/b.md"), NOTE_B);
}

#[gpui::test]
fn each_note_gets_a_fresh_editor_sharing_one_highlighter(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    std::fs::write(
        dir.path().join("code1.md"),
        "# One\n\n```rust\nfn a() {}\n```\n\n```rust\nfn b() {}\n```\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("code2.md"),
        "# Two\n\n```rust\nstruct S;\n```\n",
    )
    .unwrap();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "code1", cx);
    app.draw(cx);
    let first = editor(&app, cx);
    let languages = |cx: &mut TestAppContext| {
        let view = view(&app, cx);
        cx.read_entity(&view, |view, _| view.highlighter.languages())
    };
    assert_eq!(languages(cx), 1, "the code blocks were highlighted");

    open(&app, "code2", cx);
    app.draw(cx);
    let second = editor(&app, cx);
    assert_ne!(first.entity_id(), second.entity_id());
    assert_eq!(languages(cx), 1, "rust is parsed once for every editor");
}

#[gpui::test]
fn the_save_key_writes_at_once_and_keeps_frontmatter(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "a", cx);
    focus_at_end(&app, cx);
    type_text(&app, "!", cx);
    assert_eq!(read(dir.path(), "a.md").matches('!').count(), 0);

    keys(&app, super::SAVE_KEY, cx);

    let raw = read(dir.path(), "a.md");
    assert!(
        raw.starts_with("---\nicon: Star\nfavorite: true\n"),
        "{raw}"
    );
    assert!(raw.ends_with("---\n# A\n\nAlpha links [[b]].\n!"), "{raw}");
    let view = view(&app, cx);
    let status = cx.read_entity(&view, |view, _| view.save_status());
    assert_eq!(status, Some(SaveStatus::Clean));
}

#[gpui::test]
fn edits_are_saved_after_the_holdoff(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "b", cx);
    type_at_end(&app, "x", cx);
    cx.executor().advance_clock(AUTOSAVE_DELAY / 2);
    cx.run_until_parked();
    assert_eq!(read(dir.path(), "sub/b.md"), NOTE_B);

    // A further edit restarts the quiet period.
    type_at_end(&app, "y", cx);
    cx.executor().advance_clock(AUTOSAVE_DELAY * 3 / 4);
    cx.run_until_parked();
    assert_eq!(read(dir.path(), "sub/b.md"), NOTE_B);

    cx.executor().advance_clock(AUTOSAVE_DELAY);
    cx.run_until_parked();
    assert_eq!(read(dir.path(), "sub/b.md"), "# B\nxy");
}

#[gpui::test]
fn switching_notes_writes_pending_edits_to_the_old_note(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let note_a = read(dir.path(), "a.md");
    let app = App::with_cave(dir.path(), cx);
    open(&app, "b", cx);
    type_at_end(&app, "z", cx);
    open(&app, "a", cx);
    assert_eq!(read(dir.path(), "sub/b.md"), "# B\nz");
    assert_eq!(text(&app, cx), NOTE_A_BODY);
    // The cancelled holdoff does not write the new note.
    cx.executor().advance_clock(AUTOSAVE_DELAY * 2);
    cx.run_until_parked();
    assert_eq!(read(dir.path(), "a.md"), note_a);
}

#[gpui::test]
fn a_failed_save_keeps_the_note_open(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "b", cx);
    type_at_end(&app, "z", cx);
    // Make the write fail: the note's folder is replaced by a file.
    std::fs::remove_dir_all(dir.path().join("sub")).unwrap();
    std::fs::write(dir.path().join("sub"), "").unwrap();

    open(&app, "a", cx);

    assert_eq!(app.active_note(cx).as_deref(), Some("b"));
    assert_eq!(text(&app, cx), "# B\nz");
    let view = view(&app, cx);
    let status = cx.read_entity(&view, |view, _| view.save_status());
    assert!(matches!(status, Some(SaveStatus::Failed(_))), "{status:?}");
    app.draw(cx);
}

#[gpui::test]
fn pending_edits_are_written_on_quit(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "b", cx);
    type_at_end(&app, "q", cx);
    cx.update(|cx| cx.shutdown());
    assert_eq!(read(dir.path(), "sub/b.md"), "# B\nq");
}

#[gpui::test]
fn pending_edits_are_written_before_another_cave_opens(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let other = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "b", cx);
    type_at_end(&app, "c", cx);
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_cave(other.path().to_path_buf(), cx)
    });
    assert_eq!(read(dir.path(), "sub/b.md"), "# B\nc");
    assert_eq!(read(other.path(), "sub/b.md"), NOTE_B);
    assert_eq!(app.active_note(cx), None);
}

#[gpui::test]
fn a_cave_switch_is_refused_while_the_edits_cannot_be_saved(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let other = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "b", cx);
    type_at_end(&app, "z", cx);
    std::fs::remove_dir_all(dir.path().join("sub")).unwrap();
    std::fs::write(dir.path().join("sub"), "").unwrap();

    app.update_workspace(cx, |workspace, cx| {
        workspace.open_cave(other.path().to_path_buf(), cx)
    });

    assert_eq!(
        cx.update(|cx| Host::cave_path(cx)),
        Some(dir.path().to_path_buf())
    );
    assert_eq!(app.active_note(cx).as_deref(), Some("b"));
    assert_eq!(text(&app, cx), "# B\nz");
    let failure = cx.read_entity(&app.workspace, |workspace, _| workspace.failure());
    assert!(
        failure.is_some_and(|message| message.contains("could not be saved")),
        "the refusal is shown"
    );
    app.draw(cx);
}

#[gpui::test]
fn edits_made_while_another_cave_opens_go_to_the_old_cave(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let other = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "b", cx);
    let editor = editor(&app, cx);
    // Start the switch, then type before the new cave has been prepared.
    cx.update_window(app.window, |_, window, cx| {
        app.workspace.update(cx, |workspace, cx| {
            workspace.open_cave(other.path().to_path_buf(), cx)
        });
        editor.update(cx, |editor, cx| {
            let end = editor.text().len();
            editor.set_cursor(end, cx);
            editor.replace_text_in_range(None, "late", window, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();

    assert_eq!(
        cx.update(|cx| Host::cave_path(cx)),
        Some(other.path().to_path_buf())
    );
    assert_eq!(read(dir.path(), "sub/b.md"), "# B\nlate");
    assert_eq!(read(other.path(), "sub/b.md"), NOTE_B);
}

#[gpui::test]
fn a_clean_note_follows_changes_on_disk_and_a_dirty_one_keeps_its_edits(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "b", cx);
    let agent_writes = |body: &str, cx: &mut TestAppContext| {
        let body = body.to_string();
        cx.update(|cx| {
            Host::core(cx)
                .with_cave(|cave| cave.save_note("b", &body))
                .unwrap();
            CaveEvents::emit(CaveEvent::NotesChanged, cx);
        });
        cx.run_until_parked();
    };

    agent_writes("# B from the agent\n", cx);
    assert_eq!(text(&app, cx), "# B from the agent\n");

    type_at_end(&app, "mine", cx);
    agent_writes("# B again\n", cx);
    assert_eq!(text(&app, cx), "# B from the agent\nmine");
    flush(&app, cx);
    assert_eq!(read(dir.path(), "sub/b.md"), "# B from the agent\nmine");
}

#[gpui::test]
fn a_renamed_note_keeps_its_pending_edits(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "b", cx);
    type_at_end(&app, "!", cx);
    cx.update(|cx| cave_ops::rename_note("b", "c", cx)).unwrap();
    cx.run_until_parked();

    assert_eq!(app.active_note(cx).as_deref(), Some("c"));
    assert_eq!(text(&app, cx), "# B\n!");
    flush(&app, cx);
    assert_eq!(read(dir.path(), "sub/c.md"), "# B\n!");
}

#[gpui::test]
fn a_deleted_note_closes_without_writing(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "b", cx);
    type_at_end(&app, "!", cx);
    cx.update(|cx| cave_ops::delete_note("b", cx)).unwrap();
    cx.run_until_parked();

    assert_eq!(app.active_note(cx), None);
    assert!(!dir.path().join("sub/b.md").exists());
}

// ── Stale layout ────────────────────────────────────────────────────────

/// Window position of the painted text at byte `offset` of the open note.
fn painted_position(app: &App, offset: usize, cx: &mut TestAppContext) -> Point<Pixels> {
    let editor = editor(app, cx);
    let caret = cx
        .read_entity(&editor, |editor, _| editor.bounds_for_offset(offset))
        .expect("the offset was painted");
    point(caret.left() + px(2.), caret.top() + caret.size.height / 2.)
}

/// A cave with a note of many short lines next to the one-line `b`.
fn long_cave() -> (tempfile::TempDir, String) {
    let dir = fixture_cave();
    let long: String = (0..60).map(|i| format!("Line {i}\n")).collect();
    std::fs::write(dir.path().join("long.md"), &long).unwrap();
    (dir, long)
}

/// Run `act`, then deliver a left press at `position` before anything
/// paints again: on macOS, input queued behind a slow frame reaches the
/// views in that state.
fn press_before_paint(
    app: &App,
    position: Point<Pixels>,
    act: impl FnOnce(&App, &mut Window, &mut gpui::App),
    cx: &mut TestAppContext,
) {
    cx.update_window(app.window, |_, window, cx| {
        act(app, window, cx);
        let _ = window.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                button: MouseButton::Left,
                position,
                modifiers: Modifiers::none(),
                click_count: 1,
                first_mouse: false,
            }),
            cx,
        );
        let _ = window.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                button: MouseButton::Left,
                position,
                modifiers: Modifiers::none(),
                click_count: 1,
            }),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
}

/// A press over the previous note's lower rows, arriving after a switch to
/// a shorter note and before the next paint.
#[gpui::test]
fn a_press_before_the_switched_note_is_painted_does_not_panic(cx: &mut TestAppContext) {
    let (dir, long) = long_cave();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "long", cx);
    app.draw(cx);
    let position = painted_position(&app, long.find("Line 20").unwrap(), cx);

    press_before_paint(
        &app,
        position,
        |app, _, cx| {
            app.workspace.update(cx, |workspace, cx| {
                workspace.open_note(Some("b".into()), cx)
            })
        },
        cx,
    );
    assert_eq!(text(&app, cx), NOTE_B);
    app.draw(cx);
}

/// Within one editor, an edit that removes lines leaves the painted layout
/// behind until the next paint. zorite-editor 0.11.7 indexed past its line
/// table on a press over a removed row; the patched editor ignores it.
#[gpui::test]
fn a_press_after_an_edit_before_paint_is_ignored(cx: &mut TestAppContext) {
    let (dir, long) = long_cave();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "long", cx);
    app.draw(cx);
    let position = painted_position(&app, long.find("Line 20").unwrap(), cx);
    let editor = editor(&app, cx);

    // Select all and type over it, as the user would.
    press_before_paint(
        &app,
        position,
        |_, window, cx| {
            editor.update(cx, |editor, cx| {
                let utf16_len = editor.text().encode_utf16().count();
                editor.replace_text_in_range(Some(0..utf16_len), "x", window, cx);
            })
        },
        cx,
    );
    assert_eq!(text(&app, cx), "x");
    assert_eq!(cursor(&app, cx), 1, "the caret stays after the x");

    // The same for a host edit through `replace_range` (an accepted
    // completion), and one that keeps the line count but shortens lines.
    app.draw(cx);
    update_editor(&app, cx, |editor, _, cx| {
        let len = editor.text().len();
        editor.replace_range(0..len, &long, cx);
    });
    app.draw(cx);
    press_before_paint(
        &app,
        position,
        |_, _, cx| {
            editor.update(cx, |editor, cx| {
                let len = editor.text().len();
                editor.replace_range(0..len, &"y\n".repeat(60), cx);
            })
        },
        cx,
    );
    assert_eq!(cursor(&app, cx), 120, "the caret stays at the end");
    type_at_end(&app, "z", cx);
    assert_eq!(text(&app, cx), "y\n".repeat(60) + "z");
}

// ── Wiki-links ──────────────────────────────────────────────────────────

const LINKS: &str = "# Links\n\nPlain [[b]] and [[Räksmörgås och kaffe|fika]].\n\
                     Anchor [[volvo]] and [[Missing]].\n`[[b]]` code\n\n\
                     ```\n[[b]]\n```\n\n#tag here\n\nWeb [site](https://example.com/x).\n";

/// A cave with a link-rich note, a non-ASCII note, a note named like a tag
/// and a long note whose anchored heading is far below the fold.
fn link_cave() -> tempfile::TempDir {
    let dir = fixture_cave();
    std::fs::write(dir.path().join("links.md"), LINKS).unwrap();
    std::fs::write(dir.path().join("Räksmörgås och kaffe.md"), "# Fika\n").unwrap();
    std::fs::write(dir.path().join("tag.md"), "# Tag\n").unwrap();
    let filler = "Filler line.\n\n".repeat(150);
    std::fs::write(
        dir.path().join("long.md"),
        format!("# Long\n\n{filler}## Volvo {{#volvo}}\n\n{filler}"),
    )
    .unwrap();
    dir
}

fn open_links(cx: &mut TestAppContext) -> (tempfile::TempDir, App) {
    let dir = link_cave();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "links", cx);
    app.draw(cx);
    (dir, app)
}

/// Click the painted text of byte `offset` in the open note.
fn click_offset(app: &App, offset: usize, cx: &mut TestAppContext) {
    let position = painted_position(app, offset, cx);
    let mut vcx = VisualTestContext::from_window(app.window, cx);
    vcx.simulate_mouse_move(position, None, Modifiers::none());
    vcx.simulate_click(position, Modifiers::none());
    cx.run_until_parked();
}

fn offset_in(text: &str, needle: &str) -> usize {
    text.find(needle)
        .unwrap_or_else(|| panic!("{needle:?} not found"))
}

#[gpui::test]
fn clicking_wiki_links_opens_the_target_note(cx: &mut TestAppContext) {
    let (dir, app) = open_links(cx);
    // Inside the label of a non-ASCII, labelled link.
    click_offset(&app, offset_in(LINKS, "fika]]"), cx);
    assert_eq!(app.active_note(cx).as_deref(), Some("Räksmörgås och kaffe"));
    assert_eq!(text(&app, cx), "# Fika\n");
    assert_eq!(
        app.core_active_note(cx).as_deref(),
        Some("Räksmörgås och kaffe")
    );

    open(&app, "links", cx);
    app.draw(cx);
    click_offset(&app, offset_in(LINKS, "[[b]]") + 2, cx);
    assert_eq!(app.active_note(cx).as_deref(), Some("b"));

    // Nothing was written by navigating.
    assert_eq!(read(dir.path(), "links.md"), LINKS);
}

#[gpui::test]
fn links_in_code_and_tags_do_not_navigate(cx: &mut TestAppContext) {
    let (_dir, app) = open_links(cx);
    for needle in ["`[[b]]`", "\n[[b]]\n", "#tag"] {
        click_offset(&app, offset_in(LINKS, needle) + 3, cx);
        assert_eq!(app.active_note(cx).as_deref(), Some("links"), "{needle}");
        app.draw(cx);
    }
}

#[gpui::test]
fn broken_links_are_marked_and_create_their_note_when_clicked(cx: &mut TestAppContext) {
    let (dir, app) = open_links(cx);
    let view = view(&app, cx);
    let broken = cx.read_entity(&view, |view, cx| view.broken_links(cx));
    let missing = offset_in(LINKS, "[[Missing]]");
    assert_eq!(broken, vec![missing..missing + "[[Missing]]".len()]);

    click_offset(&app, missing + 3, cx);
    assert_eq!(app.active_note(cx).as_deref(), Some("Missing"));
    assert!(dir.path().join("Missing.md").exists());

    // Back in the linking note, the link resolves now.
    open(&app, "links", cx);
    let broken = cx.read_entity(&view, |view, cx| view.broken_links(cx));
    assert!(broken.is_empty(), "{broken:?}");
}

#[gpui::test]
fn an_anchor_link_opens_the_note_and_scrolls_to_the_heading(cx: &mut TestAppContext) {
    let (_dir, app) = open_links(cx);
    click_offset(&app, offset_in(LINKS, "[[volvo]]") + 3, cx);
    assert_eq!(app.active_note(cx).as_deref(), Some("long"));
    let heading = Markdown::link_spans(&text(&app, cx))
        .anchor_offset("volvo")
        .unwrap();
    assert_eq!(cursor(&app, cx), heading, "the caret is on the heading");

    // Frames are not scheduled in tests: the first paint lays the note
    // out, the second render scrolls.
    app.draw(cx);
    app.draw(cx);
    let view = view(&app, cx);
    let editor = editor(&app, cx);
    let (offset, viewport) = cx.read_entity(&view, |view, _| {
        (view.scroll.offset(), view.scroll.bounds())
    });
    assert!(offset.y < px(0.), "scrolled down");
    let top = cx
        .read_entity(&editor, |editor, _| editor.offset_screen_top(heading))
        .unwrap();
    assert!(
        top >= viewport.top() && top < viewport.top() + px(100.),
        "heading at {top:?}, viewport {viewport:?}"
    );
}

#[gpui::test]
fn a_note_opens_at_a_line(cx: &mut TestAppContext) {
    let (_dir, app) = open_links(cx);
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_note_at("long".into(), Some(Reveal::Line(200)), cx)
    });
    let line = wiki_links::line_offset(&text(&app, cx), 200);
    assert_eq!(cursor(&app, cx), line);
    app.draw(cx);
    app.draw(cx);
    let view = view(&app, cx);
    let offset = cx.read_entity(&view, |view, _| view.scroll.offset());
    assert!(offset.y < px(0.), "scrolled down");
}

#[test]
fn reveals_point_into_the_text() {
    let text = "# A {#a}\nline\n";
    assert_eq!(Reveal::Anchor("A".into()).offset_in(text), Some(0));
    assert_eq!(Reveal::Anchor("b".into()).offset_in(text), None);
    assert_eq!(Reveal::Line(1).offset_in(text), Some(9));
    assert_eq!(Reveal::Offset(99).offset_in(text), Some(text.len()));
    // A setext heading is an anchor too, found at its text line; a heading
    // in code is not.
    let text = "Intro\n\nSaab {#saab}\n----\n\n```\n# B {#b}\n```\n";
    assert_eq!(Reveal::Anchor("SAAB".into()).offset_in(text), Some(7));
    assert_eq!(Reveal::Anchor("b".into()).offset_in(text), None);
}

#[gpui::test]
fn a_note_opens_at_a_setext_anchor(cx: &mut TestAppContext) {
    let dir = link_cave();
    let filler = "Filler line.\n\n".repeat(150);
    let body = format!("# Setext\n\n{filler}Saab {{#saab}}\n----\n\n{filler}");
    std::fs::write(dir.path().join("setext.md"), &body).unwrap();
    let app = App::with_cave(dir.path(), cx);
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_note_at("setext".into(), Some(Reveal::Anchor("saab".into())), cx)
    });
    assert_eq!(cursor(&app, cx), offset_in(&body, "Saab {#saab}"));
    app.draw(cx);
    app.draw(cx);
    let view = view(&app, cx);
    let offset = cx.read_entity(&view, |view, _| view.scroll.offset());
    assert!(offset.y < px(0.), "scrolled down");
}

#[gpui::test]
fn external_links_open_through_the_platform(cx: &mut TestAppContext) {
    let (_dir, app) = open_links(cx);
    click_offset(&app, offset_in(LINKS, "site]") + 1, cx);
    assert_eq!(cx.opened_url().as_deref(), Some("https://example.com/x"));
    assert_eq!(app.active_note(cx).as_deref(), Some("links"));
}

// ── Completion ──────────────────────────────────────────────────────────

fn completion_labels(app: &App, cx: &mut TestAppContext) -> Option<Vec<String>> {
    let view = view(app, cx);
    cx.read_entity(&view, |view, _| {
        view.completion()
            .map(|c| c.labels().into_iter().map(str::to_string).collect())
    })
}

#[gpui::test]
fn typing_double_brackets_offers_notes_and_anchors(cx: &mut TestAppContext) {
    let (_dir, app) = open_links(cx);
    focus_at_end(&app, cx);
    type_text(&app, "[[", cx);
    let labels = completion_labels(&app, cx).expect("popup open");
    for expected in [
        "a",
        "b",
        "links",
        "long",
        "Räksmörgås och kaffe",
        "volvo",
        "Missing",
    ] {
        assert!(
            labels.iter().any(|l| l == expected),
            "{expected} in {labels:?}"
        );
    }
    // The popup is anchored at the painted `[[`.
    let view = view(&app, cx);
    cx.update_window(app.window, |_, _, cx| {
        view.update(cx, |view, cx| assert!(view.render_completion(cx).is_some()))
    })
    .unwrap();

    type_text(&app, "räk", cx);
    assert_eq!(
        completion_labels(&app, cx).unwrap(),
        ["Räksmörgås och kaffe", "räk"]
    );
    keys(&app, "enter", cx);
    assert!(completion_labels(&app, cx).is_none(), "closed after accept");
    let text = text(&app, cx);
    assert!(text.ends_with("[[Räksmörgås och kaffe]]"), "{text:?}");
    // The caret sits after the closing brackets: typing continues there.
    type_text(&app, "!", cx);
    assert!(self::text(&app, cx).ends_with("kaffe]]!"));
}

#[gpui::test]
fn arrows_select_tab_accepts_and_escape_dismisses(cx: &mut TestAppContext) {
    let (dir, app) = open_links(cx);
    focus_at_end(&app, cx);
    let before = text(&app, cx);
    type_text(&app, "[[lo", cx);
    assert_eq!(
        completion_labels(&app, cx).unwrap(),
        ["long", "volvo", "lo"]
    );
    keys(&app, "down", cx);
    let view = view(&app, cx);
    let selected = cx.read_entity(&view, |view, _| view.completion().unwrap().selected());
    assert_eq!(selected, 1);
    // The arrow moved the selection, not the caret.
    assert_eq!(text(&app, cx), format!("{before}[[lo"));
    keys(&app, "up up", cx);
    keys(&app, "down tab", cx);
    assert_eq!(text(&app, cx), format!("{before}[[long]]"));

    type_text(&app, " [[b", cx);
    assert!(completion_labels(&app, cx).is_some());
    keys(&app, "escape", cx);
    assert!(completion_labels(&app, cx).is_none());
    // Dismissed stays dismissed while typing in the same link; Enter edits
    // again.
    type_text(&app, "x", cx);
    assert!(completion_labels(&app, cx).is_none());
    keys(&app, "enter", cx);
    assert_eq!(text(&app, cx), format!("{before}[[long]] [[bx\n"));

    // The accepted completion is saved like typed text.
    cx.executor().advance_clock(AUTOSAVE_DELAY * 2);
    cx.run_until_parked();
    let saved = read(dir.path(), "links.md");
    assert!(saved.ends_with("[[long]] [[bx\n"), "{saved:?}");
}

#[gpui::test]
fn no_completion_inside_code(cx: &mut TestAppContext) {
    let (_dir, app) = open_links(cx);
    focus_at_end(&app, cx);
    // An unclosed backtick is not code yet, so this completes...
    type_text(&app, "`[[", cx);
    assert!(completion_labels(&app, cx).is_some());
    // ...until the span is closed around the link.
    type_text(&app, "x`", cx);
    assert!(completion_labels(&app, cx).is_none());
    keys(&app, "left", cx);
    assert!(completion_labels(&app, cx).is_none());
    // Indented code after a blank line.
    keys(&app, "end enter enter", cx);
    type_text(&app, "    [[", cx);
    let text = text(&app, cx);
    assert!(text.ends_with("\n\n    [["), "{text:?}");
    assert!(
        completion_labels(&app, cx).is_none(),
        "inside indented code"
    );
}

#[gpui::test]
fn the_link_and_task_keys_edit_like_the_web_editor(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "b", cx);
    focus_at_end(&app, cx);
    keys(&app, super::TOGGLE_TASK_KEY, cx);
    assert_eq!(text(&app, cx), "# B\n- [ ] ");
    keys(&app, super::INSERT_LINK_KEY, cx);
    assert_eq!(text(&app, cx), "# B\n- [ ] [[]]");
    assert!(
        completion_labels(&app, cx).is_some(),
        "link completion opens"
    );
    keys(&app, super::TOGGLE_TASK_KEY, cx);
    assert_eq!(text(&app, cx), "# B\n- [x] [[]]");
    flush(&app, cx);
    assert_eq!(read(dir.path(), "sub/b.md"), "# B\n- [x] [[]]");
}

// ── Find ────────────────────────────────────────────────────────────────

#[gpui::test]
fn find_highlights_and_steps_through_matches(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    std::fs::write(
        dir.path().join("find.md"),
        "Volvo one\n\nvolvo two\n\nVOLVO three\n",
    )
    .unwrap();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "find", cx);
    focus_at(&app, 0, cx);
    keys(&app, super::FIND_KEY, cx);
    let view = view(&app, cx);
    assert!(cx.read_entity(&view, |view, _| view.find_state().is_some()));
    let focused = cx
        .update_window(app.window, |_, window, cx| {
            view.read(cx).find_input_focus(window, cx)
        })
        .unwrap();
    assert!(focused, "the find field has focus");

    update_view(&app, cx, |view, window, cx| {
        view.set_find_query("volvo", window, cx)
    });
    let state = cx.read_entity(&view, |view, _| view.find_state()).unwrap();
    assert_eq!(state.0, [0..5, 11..16, 22..27]);
    assert_eq!(state.1, Some(0));
    app.draw(cx);

    keys(&app, "enter", cx);
    assert_eq!(
        cx.read_entity(&view, |v, _| v.find_state()).unwrap().1,
        Some(1)
    );
    keys(&app, "shift-enter", cx);
    keys(&app, "shift-enter", cx);
    assert_eq!(
        cx.read_entity(&view, |v, _| v.find_state()).unwrap().1,
        Some(2)
    );

    // An edit updates the matches.
    type_at_end(&app, "volvo", cx);
    assert_eq!(
        cx.read_entity(&view, |v, _| v.find_state())
            .unwrap()
            .0
            .len(),
        4
    );

    keys(&app, "escape", cx);
    assert!(cx.read_entity(&view, |view, _| view.find_state().is_none()));
    let editor = editor(&app, cx);
    let focused = cx
        .update_window(app.window, |_, window, cx| {
            editor.read(cx).focus_handle(cx).is_focused(window)
        })
        .unwrap();
    assert!(focused, "the editor has focus again");
}

// ── Images ──────────────────────────────────────────────────────────────

#[gpui::test]
fn relative_images_load_from_the_note_directory(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    std::fs::create_dir_all(dir.path().join("sub/img")).unwrap();
    std::fs::write(
        dir.path().join("sub/img/pic.svg"),
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="160"><rect width="200" height="160" fill="red"/></svg>"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("sub/pics.md"),
        "# Pics\n\n![pic](img/pic.svg)\n\nText\n",
    )
    .unwrap();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "pics", cx);
    cx.run_until_parked();
    let view = view(&app, cx);
    let path = dir.path().join("sub/img/pic.svg");
    assert!(
        cx.read_entity(&view, |view, _| view.image_cache.get(&path).is_some()),
        "the image was decoded"
    );

    app.draw(cx);
    let editor = editor(&app, cx);
    let rows = cx.read_entity(&editor, |editor, _| editor.row_layout());
    let (image_row, text_row) = (rows[2].1, rows[4].1);
    assert!(
        image_row > text_row * 2.,
        "the image line is drawn as the image: {rows:?}"
    );
}

#[gpui::test]
fn images_follow_a_moved_note(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let svg = |side: u32| {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{side}" height="{side}"><rect width="{side}" height="{side}" fill="red"/></svg>"#
        )
    };
    for folder in ["sub/img", "img"] {
        std::fs::create_dir_all(dir.path().join(folder)).unwrap();
    }
    std::fs::write(dir.path().join("sub/img/pic.svg"), svg(100)).unwrap();
    std::fs::write(dir.path().join("img/pic.svg"), svg(200)).unwrap();
    std::fs::write(dir.path().join("sub/pics.md"), "![pic](img/pic.svg)\n").unwrap();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "pics", cx);
    let view = view(&app, cx);
    let root_image = dir.path().join("img/pic.svg");
    assert!(cx.read_entity(&view, |view, _| view.image_cache.get(&root_image).is_none()));

    cx.update(|cx| cave_ops::move_note("pics", None, cx))
        .unwrap();
    cx.run_until_parked();

    assert!(
        cx.read_entity(&view, |view, _| view.image_cache.get(&root_image).is_some()),
        "the image is resolved from the note's new folder"
    );
    app.draw(cx);
}

// ── Round-trip ──────────────────────────────────────────────────────────

/// The edge-case fixtures of the core crate's round-trip tests, opened in
/// the editor: the editor reads every body back byte for byte, and writing
/// that text leaves each file byte-identical.
#[gpui::test]
fn the_round_trip_fixtures_survive_the_editor_byte_for_byte(cx: &mut TestAppContext) {
    let fixtures =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../granit-core/tests/fixtures/roundtrip");
    let dir = tempfile::tempdir().unwrap();
    let mut notes = Vec::new();
    for entry in std::fs::read_dir(&fixtures).unwrap() {
        let path = entry.unwrap().path();
        let bytes = std::fs::read(&path).unwrap();
        std::fs::write(dir.path().join(path.file_name().unwrap()), &bytes).unwrap();
        let slug = path.file_stem().unwrap().to_string_lossy().into_owned();
        notes.push((slug, path.file_name().unwrap().to_owned(), bytes));
    }
    notes.sort();
    assert!(notes.len() >= 10, "fixtures missing");

    let app = App::with_cave(dir.path(), cx);
    for (slug, file, original) in &notes {
        open(&app, slug, cx);
        app.draw(cx);
        let body = cx
            .update(|cx| cave_ops::read_note(slug, cx))
            .unwrap()
            .content;
        let shown = text(&app, cx);
        assert_eq!(shown, body, "{slug}: the editor changed the body on load");

        // An unedited save writes nothing; a save of the editor's text
        // writes the same bytes.
        assert_eq!(flush(&app, cx), FlushOutcome::Unchanged, "{slug}");
        cx.update(|cx| cave_ops::save_note(slug, &shown, cx))
            .unwrap();
        let after = std::fs::read(dir.path().join(file)).unwrap();
        assert!(after == *original, "{slug}: the file changed on save");
    }
}

#[gpui::test]
fn display_math_sharing_a_line_with_words_stays_put_while_typing(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let body = "Words before $$x^2$$ words after\n";
    std::fs::write(dir.path().join("math.md"), body).unwrap();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "math", cx);
    assert_eq!(text(&app, cx), body);
    focus_at(&app, body.len() - 1, cx);
    type_text(&app, " more.", cx);
    assert_eq!(text(&app, cx), "Words before $$x^2$$ words after more.\n");
}

#[gpui::test]
fn undo_never_applies_the_editors_load_normalization(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let body = "Words before $$x^2$$ words after\n";
    std::fs::write(dir.path().join("math.md"), body).unwrap();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "math", cx);
    focus_at(&app, body.len() - 1, cx);

    // Right after opening: nothing to undo.
    keys(&app, "ctrl-z", cx);
    assert_eq!(text(&app, cx), body);
    cx.executor().advance_clock(AUTOSAVE_DELAY * 2);
    cx.run_until_parked();
    assert_eq!(read(dir.path(), "math.md"), body);
    let view = view(&app, cx);
    let status = cx.read_entity(&view, |view, _| view.save_status());
    assert_eq!(status, Some(SaveStatus::Clean));

    // A real edit undoes back to the loaded body, and no further.
    type_text(&app, "!", cx);
    keys(&app, "ctrl-z", cx);
    assert_eq!(text(&app, cx), body);
    keys(&app, "ctrl-z", cx);
    assert_eq!(text(&app, cx), body);
    cx.executor().advance_clock(AUTOSAVE_DELAY * 2);
    cx.run_until_parked();
    assert_eq!(read(dir.path(), "math.md"), body);
}

#[gpui::test]
fn enter_in_a_crlf_note_writes_crlf(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let body = "# CRLF\r\n\r\nfirst\r\nsecond\r\n";
    std::fs::write(dir.path().join("crlf.md"), body).unwrap();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "crlf", cx);
    assert_eq!(text(&app, cx), body);
    focus_at(&app, offset_in(body, "first") + "first".len(), cx);
    keys(&app, "enter", cx);
    type_text(&app, "x", cx);
    flush(&app, cx);
    assert_eq!(
        read(dir.path(), "crlf.md"),
        "# CRLF\r\n\r\nfirst\r\nx\r\nsecond\r\n"
    );
}

const TEMPLATE: &str = "---\nicon: Star\n# keep me\ntags: [day]\n---\n# {{ weekday }} {{ date }}\n\n{% if day == 1 %}New month!{% endif %}\n\n[[{{ yesterday }}]] · [[b]]\n";
const TEMPLATE_BODY: &str =
    "# {{ weekday }} {{ date }}\n\n{% if day == 1 %}New month!{% endif %}\n\n[[{{ yesterday }}]] · [[b]]\n";

fn template_cave() -> tempfile::TempDir {
    let dir = fixture_cave();
    std::fs::create_dir_all(dir.path().join(".granit/templates")).unwrap();
    std::fs::write(dir.path().join(".granit/templates/day.md"), TEMPLATE).unwrap();
    // A note of the same name is another document.
    std::fs::write(dir.path().join("day.md"), "# The note\n").unwrap();
    dir
}

fn open_template(app: &App, slug: &str, cx: &mut TestAppContext) {
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_template(slug.to_string(), cx)
    });
}

#[gpui::test]
fn a_template_round_trips_through_the_editor(cx: &mut TestAppContext) {
    let dir = template_cave();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "a", cx);
    open_template(&app, "day", cx);
    app.draw(cx);

    assert_eq!(
        text(&app, cx),
        TEMPLATE_BODY,
        "Tera syntax loads as written"
    );
    assert_eq!(app.active_note(cx), None, "no note is active");
    let view = view(&app, cx);
    let broken = cx.read_entity(&view, |view, cx| view.broken_links(cx));
    assert!(
        broken.is_empty(),
        "placeholder links are not broken: {broken:?}"
    );

    // Unedited: switching away and back writes nothing.
    open(&app, "day", cx);
    assert_eq!(text(&app, cx), "# The note\n", "the note of that name");
    open_template(&app, "day", cx);
    cx.executor().advance_clock(AUTOSAVE_DELAY * 2);
    cx.run_until_parked();
    assert_eq!(read(dir.path(), ".granit/templates/day.md"), TEMPLATE);
    assert_eq!(flush(&app, cx), FlushOutcome::Unchanged);
    let shown = text(&app, cx);
    cx.update(|cx| cave_ops::save_template("day", &shown, cx))
        .unwrap();
    assert_eq!(read(dir.path(), ".granit/templates/day.md"), TEMPLATE);

    // Edited: autosaved to the template, frontmatter kept, the note of the
    // same name untouched.
    type_at_end(&app, "{{ slug }}", cx);
    cx.executor().advance_clock(AUTOSAVE_DELAY * 2);
    cx.run_until_parked();
    // (The core stamps `modified_at`, as for notes and in the Tauri app.)
    let raw = read(dir.path(), ".granit/templates/day.md");
    assert!(
        raw.starts_with("---\nicon: Star\n# keep me\ntags: [day]\nmodified_at: "),
        "{raw}"
    );
    assert!(
        raw.ends_with(&format!("---\n{TEMPLATE_BODY}{{{{ slug }}}}")),
        "{raw}"
    );
    assert_eq!(read(dir.path(), "day.md"), "# The note\n");

    // Pending edits are written before a note opens.
    type_at_end(&app, "!", cx);
    open(&app, "b", cx);
    assert!(read(dir.path(), ".granit/templates/day.md").ends_with("{{ slug }}!"));
    assert_eq!(read(dir.path(), "day.md"), "# The note\n");

    // A note made from it is rendered from the saved template.
    let body = cx
        .update(|cx| {
            Host::core(cx).with_cave(|cave| cave.render_note_template("day", "2026-12-01"))
        })
        .unwrap();
    assert_eq!(
        body,
        "# Tuesday 2026-12-01\n\nNew month!\n\n[[2026-11-30]] · [[b]]\n2026-12-01!"
    );
}

#[gpui::test]
fn a_broken_link_in_a_template_creates_no_note(cx: &mut TestAppContext) {
    let dir = template_cave();
    let app = App::with_cave(dir.path(), cx);
    open_template(&app, "day", cx);
    app.draw(cx);
    update_view(&app, cx, |view, _, cx| {
        view.navigate_to_link("{{ yesterday }}", cx)
    });
    assert!(!dir.path().join("{{ yesterday }}.md").exists());
    let shown = cx.read_entity(&app.workspace, |workspace, cx| {
        workspace.open_template_slug(cx)
    });
    assert_eq!(shown.as_deref(), Some("day"));

    // A link to an existing note still opens it.
    update_view(&app, cx, |view, _, cx| view.navigate_to_link("b", cx));
    assert_eq!(app.active_note(cx).as_deref(), Some("b"));
}

// ── Agent skills and tasks ──────────────────────────────────────────────

const SKILL: &str = "---\n# written by hand\nname: helper\ndescription: \"Old: help\"\nlicense: MIT\n---\n\nUse [[b]] and [[{{ active_note }}]].\n";
const SKILL_BODY: &str = "Use [[b]] and [[{{ active_note }}]].\n";
const TASK: &str =
    "---\ndescription: Sum up\n---\n\nSummarize {{ input }} for [[{{ active_note }}]].\n";

fn agent_cave() -> tempfile::TempDir {
    let dir = fixture_cave();
    let skill = dir.path().join(".granit/agent/skills/helper");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(skill.join("SKILL.md"), SKILL).unwrap();
    std::fs::create_dir_all(dir.path().join(".granit/agent/tasks")).unwrap();
    std::fs::write(dir.path().join(".granit/agent/tasks/summarize.md"), TASK).unwrap();
    dir
}

fn open_agent_doc(app: &App, kind: AgentDocKind, name: &str, cx: &mut TestAppContext) {
    app.update_workspace(cx, |workspace, cx| {
        workspace.open_agent_doc(kind, name.to_string(), cx)
    });
}

fn description(app: &App, cx: &mut TestAppContext) -> Option<String> {
    let view = view(app, cx);
    cx.read_entity(&view, |view, cx| view.description_text(cx))
}

#[gpui::test]
async fn a_skill_round_trips_through_the_editor(cx: &mut TestAppContext) {
    let dir = agent_cave();
    let path = ".granit/agent/skills/helper/SKILL.md";
    let app = App::with_cave(dir.path(), cx);
    open(&app, "a", cx);
    open_agent_doc(&app, AgentDocKind::Skill, "helper", cx);
    app.draw(cx);

    assert_eq!(text(&app, cx), SKILL_BODY, "the body, without frontmatter");
    assert_eq!(app.active_note(cx), None, "no note is active");
    assert_eq!(description(&app, cx).as_deref(), Some("Old: help"));
    let view = view(&app, cx);
    // Skills are never rendered with Tera: an unresolved link is broken,
    // as in a note.
    let broken = cx.read_entity(&view, |view, cx| view.broken_links(cx));
    assert_eq!(broken.len(), 1, "{broken:?}");
    assert!(
        SKILL_BODY[broken[0].clone()].contains("{{ active_note }}"),
        "{broken:?}"
    );

    // Unedited: switching away and back writes nothing, nor resets the agent.
    let generation = cx.update(|cx| Host::core(cx).agent_generation());
    open(&app, "b", cx);
    open_agent_doc(&app, AgentDocKind::Skill, "helper", cx);
    cx.executor().advance_clock(AUTOSAVE_DELAY * 2);
    cx.run_until_parked();
    assert_eq!(read(dir.path(), path), SKILL);
    assert_eq!(flush(&app, cx), FlushOutcome::Unchanged);
    assert_eq!(
        cx.update(|cx| Host::core(cx).agent_generation()),
        generation
    );

    // Something in the chat, to see that a skill edit leaves it.
    let chat = cx.read_entity(&app.workspace, |workspace, _| workspace.chat().clone());
    let items = vec![
        Ok(AgentStreamItem::Text("Hello there.".into())),
        Ok(AgentStreamItem::Done),
    ];
    let task = chat.update(cx, |chat, cx| {
        chat.run_stream(
            async move { Ok(AgentStream::from_items(items)) },
            "hi".into(),
            generation,
            cx,
        );
        chat.take_stream().unwrap()
    });
    // The stream is read on the tokio runtime, outside GPUI's test
    // executor.
    cx.executor().allow_parking();
    task.await;
    cx.run_until_parked();
    let transcript = cx.read_entity(&chat, |chat, _| chat.transcript().to_string());
    assert!(transcript.contains("Hello there."), "{transcript}");

    // Edited: autosaved under the frontmatter, kept byte for byte, and the
    // agent is reset (as the Tauri app's update_skill does), while the chat
    // keeps its transcript.
    type_at_end(&app, "Then stop.\n", cx);
    cx.executor().advance_clock(AUTOSAVE_DELAY * 2);
    cx.run_until_parked();
    assert_eq!(
        read(dir.path(), path),
        format!("{SKILL}Then stop.\n"),
        "only the body changed"
    );
    assert!(cx.update(|cx| Host::core(cx).agent_generation()) > generation);
    assert_eq!(
        cx.read_entity(&chat, |chat, _| chat.transcript().to_string()),
        transcript,
        "the chat stays"
    );

    // The description: Enter writes just that key.
    update_view(&app, cx, |view, window, cx| {
        view.type_description("  Help with notes: all of them ", window, cx);
        view.commit_description(cx)
    });
    assert_eq!(
        read(dir.path(), path),
        format!("{SKILL}Then stop.\n").replace("\"Old: help\"", "\"Help with notes: all of them\"")
    );
    let listed = cx.update(|cx| cave_ops::list_agent_docs(AgentDocKind::Skill, cx).unwrap());
    assert_eq!(listed[0].description, "Help with notes: all of them");

    // A description typed but not committed is written before another
    // document opens, like pending body edits.
    update_view(&app, cx, |view, window, cx| {
        view.type_description("Helps", window, cx)
    });
    type_at_end(&app, "!", cx);
    open(&app, "a", cx);
    let raw = read(dir.path(), path);
    assert!(raw.contains("\ndescription: \"Helps\"\n"), "{raw}");
    assert!(raw.ends_with("Then stop.\n!"), "{raw}");
    assert_eq!(app.active_note(cx).as_deref(), Some("a"));

    // A broken link in a skill creates no note; one to a note opens it.
    open_agent_doc(&app, AgentDocKind::Skill, "helper", cx);
    app.draw(cx);
    update_view(&app, cx, |view, _, cx| {
        view.navigate_to_link("{{ active_note }}", cx)
    });
    assert!(!dir.path().join("{{ active_note }}.md").exists());
    update_view(&app, cx, |view, _, cx| view.navigate_to_link("b", cx));
    assert_eq!(app.active_note(cx).as_deref(), Some("b"));
}

#[gpui::test]
fn a_task_is_edited_and_renamed_in_the_editor(cx: &mut TestAppContext) {
    let dir = agent_cave();
    let app = App::with_cave(dir.path(), cx);
    open_agent_doc(&app, AgentDocKind::Task, "summarize", cx);
    app.draw(cx);
    assert_eq!(
        text(&app, cx),
        "Summarize {{ input }} for [[{{ active_note }}]].\n"
    );
    assert_eq!(description(&app, cx).as_deref(), Some("Sum up"));
    let generation = cx.update(|cx| Host::core(cx).agent_generation());

    // Pending edits follow a rename from the header's title.
    type_at_end(&app, "Be brief.", cx);
    update_view(&app, cx, |view, window, cx| {
        view.start_title_edit(window, cx);
        view.type_title("brief", window, cx);
        view.commit_title(cx);
    });
    let tasks = dir.path().join(".granit/agent/tasks");
    assert!(!tasks.join("summarize.md").exists());
    let shown = cx.read_entity(&app.workspace, |workspace, cx| {
        workspace.open_agent_doc_name(cx)
    });
    assert_eq!(shown, Some((AgentDocKind::Task, "brief".into())));
    assert_eq!(flush(&app, cx), FlushOutcome::Saved);
    assert_eq!(read(&tasks, "brief.md"), format!("{TASK}Be brief."));

    update_view(&app, cx, |view, window, cx| {
        view.type_description("Summarize briefly", window, cx);
        view.commit_description(cx)
    });
    assert_eq!(
        read(&tasks, "brief.md"),
        format!("{TASK}Be brief.").replace("Sum up", "Summarize briefly")
    );
    assert_eq!(
        cx.update(|cx| Host::core(cx).agent_generation()),
        generation,
        "tasks do not reset the agent"
    );

    // The rendered prompt is the saved one.
    let prompt = cx
        .update(|cx| crate::agent_ops::render_task("brief", "the notes", cx))
        .unwrap_err();
    assert!(prompt.contains("active_note"), "no note is open: {prompt}");
    open(&app, "a", cx);
    let prompt = cx
        .update(|cx| crate::agent_ops::render_task("brief", "the notes", cx))
        .unwrap();
    assert_eq!(prompt, "Summarize the notes for [[a]].\nBe brief.");
}

// ── Header ──────────────────────────────────────────────────────────────

fn header(app: &App, cx: &mut TestAppContext) -> DocumentHeader {
    let view = view(app, cx);
    cx.read_entity(&view, |view, _| view.header_info().cloned())
        .expect("the header is loaded")
}

fn header_tags(app: &App, cx: &mut TestAppContext) -> Vec<String> {
    header(app, cx).tags
}

/// The explorer's (icon path, favorite) of the note `slug`.
fn explorer_decoration(app: &App, slug: &str, cx: &mut TestAppContext) -> (String, bool) {
    let explorer = app.explorer(cx);
    let decorations = cx.read_entity(&explorer, |explorer, cx| explorer.note_decorations(cx));
    let (_, decoration) = decorations
        .into_iter()
        .find(|(shown, _)| shown == slug)
        .expect("the note is shown");
    (decoration.icon.to_string(), decoration.favorite)
}

#[gpui::test]
fn the_favorite_star_changes_only_favorite_and_keeps_typed_text(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "a", cx);
    app.draw(cx);
    assert!(header(&app, cx).favorite);
    type_at_end(&app, "typed", cx);

    update_view(&app, cx, |view, _, cx| view.toggle_favorite(cx));

    let raw = read(dir.path(), "a.md");
    assert!(
        raw.starts_with("---\nicon: Star\nfavorite: false\n"),
        "only the favorite key changed: {raw}"
    );
    assert!(raw.contains("\nmodified_at: "), "{raw}");
    assert!(
        raw.ends_with("---\n# A\n\nAlpha links [[b]].\ntyped"),
        "the typed text was written first: {raw}"
    );
    assert_eq!(text(&app, cx), "# A\n\nAlpha links [[b]].\ntyped");
    assert!(!header(&app, cx).favorite);
    assert_eq!(
        explorer_decoration(&app, "a", cx),
        ("note-icons/Star.svg".into(), false)
    );

    update_view(&app, cx, |view, _, cx| view.toggle_favorite(cx));
    assert!(read(dir.path(), "a.md").starts_with("---\nicon: Star\nfavorite: true\n"));
    assert!(header(&app, cx).favorite);
    assert!(
        explorer_decoration(&app, "a", cx).1,
        "the explorer star is back"
    );
    app.draw(cx);
}

#[gpui::test]
fn a_header_edit_whose_typed_text_cannot_be_saved_changes_nothing(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "b", cx);
    type_at_end(&app, "z", cx);
    std::fs::remove_dir_all(dir.path().join("sub")).unwrap();
    std::fs::write(dir.path().join("sub"), "").unwrap();

    update_view(&app, cx, |view, _, cx| view.toggle_favorite(cx));

    let view = view(&app, cx);
    let error = cx.read_entity(&view, |view, _| view.header_error().cloned());
    assert!(
        error.is_some_and(|error| error.contains("could not be saved")),
        "the refusal is shown"
    );
    assert_eq!(text(&app, cx), "# B\nz", "the typed text is kept");
    app.draw(cx);
}

#[gpui::test]
fn the_icon_picker_searches_sets_and_removes_the_icon(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "a", cx);
    update_view(&app, cx, |view, window, cx| {
        view.open_icon_picker(window, cx)
    });
    app.draw(cx);

    let shown = update_view(&app, cx, |view, window, cx| {
        view.search_icons("ALARM", window, cx)
    })
    .expect("the picker is open");
    assert!(shown.contains(&"AlarmClock"), "{shown:?}");
    assert!(shown.len() < granit_types::NOTE_ICONS.len());
    app.draw(cx);

    update_view(&app, cx, |view, _, cx| {
        view.choose_icon(Some("AlarmClock".into()), cx)
    });
    let view = view(&app, cx);
    assert!(!cx.read_entity(&view, |view, _| view.icon_picker_open()));
    assert!(
        read(dir.path(), "a.md").starts_with("---\nicon: AlarmClock\nfavorite: true\n"),
        "only the icon key changed"
    );
    assert_eq!(header(&app, cx).icon.as_deref(), Some("AlarmClock"));
    assert_eq!(
        explorer_decoration(&app, "a", cx),
        ("note-icons/AlarmClock.svg".into(), true)
    );

    update_view(&app, cx, |view, _, cx| view.choose_icon(None, cx));
    let raw = read(dir.path(), "a.md");
    assert!(raw.starts_with("---\nfavorite: true\n"), "{raw}");
    assert!(!raw.contains("icon:"), "{raw}");
    assert_eq!(header(&app, cx).icon, None);
    app.draw(cx);
}

#[gpui::test]
fn clicking_the_icon_button_opens_the_picker_and_outside_closes_it(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "a", cx);
    app.draw(cx);
    let view = view(&app, cx);

    let mut vcx = VisualTestContext::from_window(app.window, cx);
    let trigger = vcx
        .debug_bounds("note-icon-trigger")
        .expect("the icon button is drawn");
    assert!(vcx.debug_bounds("note-icon-grid").is_none());
    vcx.simulate_click(trigger.center(), Modifiers::none());
    vcx.run_until_parked();
    let _ = vcx.update(|window, cx| window.draw(cx));
    let _ = vcx.update(|window, cx| window.draw(cx));

    assert!(
        vcx.read_entity(&view, |view, _| view.icon_picker_open()),
        "the trigger opened the picker"
    );
    assert!(
        vcx.debug_bounds("note-icon-grid").is_some(),
        "the icon grid is shown"
    );

    // A press outside the popover dismisses it, and the view follows.
    let far = trigger.center() + point(px(400.), px(400.));
    vcx.simulate_click(far, Modifiers::none());
    vcx.run_until_parked();
    let _ = vcx.update(|window, cx| window.draw(cx));
    assert!(!vcx.read_entity(&view, |view, _| view.icon_picker_open()));
    assert!(vcx.debug_bounds("note-icon-grid").is_none());
}

#[gpui::test]
fn tags_are_added_on_enter_or_a_comma_and_removed_from_their_pill(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "a", cx);

    update_view(&app, cx, |view, window, cx| {
        view.type_tag("Work, pla", false, window, cx)
    });
    assert_eq!(header_tags(&app, cx), vec!["work"]);
    let view = view(&app, cx);
    assert_eq!(
        cx.read_entity(&view, |view, cx| view.tag_input_text(cx)),
        "pla"
    );

    update_view(&app, cx, |view, window, cx| {
        view.type_tag("Plan", true, window, cx)
    });
    // A tag already there is not added twice.
    update_view(&app, cx, |view, window, cx| {
        view.type_tag(" WORK ", true, window, cx)
    });
    assert_eq!(header_tags(&app, cx), vec!["work", "plan"]);
    assert_eq!(
        cx.read_entity(&view, |view, cx| view.tag_input_text(cx)),
        ""
    );
    let raw = read(dir.path(), "a.md");
    assert!(
        raw.starts_with("---\nicon: Star\nfavorite: true\n"),
        "the other keys are untouched: {raw}"
    );
    assert!(raw.ends_with("---\n# A\n\nAlpha links [[b]].\n"), "{raw}");
    app.draw(cx);

    update_view(&app, cx, |view, _, cx| {
        view.edit_header(HeaderEdit::RemoveTag("work".into()), cx)
    });
    assert_eq!(header_tags(&app, cx), vec!["plan"]);
    let tags = cx
        .update(|cx| Host::core(cx).with_cave(|cave| cave.list_tags()))
        .unwrap();
    assert!(tags.tags.contains_key("plan") && !tags.tags.contains_key("work"));
}

#[gpui::test]
fn the_header_shows_dates_word_count_and_reading_time(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    std::fs::write(
        dir.path().join("dated.md"),
        "---\ncreated_at: 2026-01-02T03:04:05Z\nmodified_at: 2026-01-03T03:04:05Z\n---\none two three\n",
    )
    .unwrap();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "dated", cx);
    app.draw(cx);

    let shown = header(&app, cx).stats;
    let expected = cx.update(|cx| {
        Host::core(cx)
            .with_cave(|cave| Ok(Markdown::new(&cave.read_note_raw("dated")?).stats()))
            .unwrap()
    });
    assert_eq!(shown, expected, "the core's dates and counts");
    assert!(shown.created_display.is_some() && shown.modified_display.is_some());
    assert_eq!((shown.word_count, shown.reading_minutes), (3, 1));

    // A save updates the count and the modified date.
    type_at_end(&app, "four", cx);
    flush(&app, cx);
    let saved = header(&app, cx).stats;
    assert_eq!(saved.word_count, 4);
    assert_ne!(saved.modified_display, shown.modified_display);
    assert_eq!(saved.created_display, shown.created_display);
    app.draw(cx);
}

#[gpui::test]
fn editing_the_title_renames_the_note_and_keeps_typed_text(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "b", cx);
    type_at_end(&app, "!", cx);

    // Escape (cancel) and a blank title rename nothing.
    update_view(&app, cx, |view, window, cx| {
        view.start_title_edit(window, cx)
    });
    app.draw(cx);
    update_view(&app, cx, |view, _, cx| view.cancel_title_edit(cx));
    update_view(&app, cx, |view, window, cx| {
        view.start_title_edit(window, cx);
        view.type_title("  ", window, cx);
        view.commit_title(cx);
    });
    assert_eq!(app.active_note(cx).as_deref(), Some("b"));

    update_view(&app, cx, |view, window, cx| {
        view.start_title_edit(window, cx);
        view.type_title("renamed", window, cx);
        view.commit_title(cx);
    });

    assert_eq!(app.active_note(cx).as_deref(), Some("renamed"));
    assert!(!dir.path().join("sub/b.md").exists());
    assert_eq!(text(&app, cx), "# B\n!", "the typed text follows the note");
    assert!(
        read(dir.path(), "a.md").contains("[[renamed]]"),
        "links follow"
    );
    flush(&app, cx);
    assert_eq!(read(dir.path(), "sub/renamed.md"), "# B\n!");

    // A taken name is refused and shown.
    update_view(&app, cx, |view, window, cx| {
        view.start_title_edit(window, cx);
        view.type_title("a", window, cx);
        view.commit_title(cx);
    });
    assert_eq!(app.active_note(cx).as_deref(), Some("renamed"));
    let view = view(&app, cx);
    assert!(cx.read_entity(&view, |view, _| view.header_error().is_some()));
    app.draw(cx);
}

#[gpui::test]
fn the_backlinks_section_lists_and_opens_the_linking_notes(cx: &mut TestAppContext) {
    let dir = fixture_cave();
    let app = App::with_cave(dir.path(), cx);
    open(&app, "b", cx);
    let backlinks: Vec<String> = header(&app, cx)
        .backlinks
        .into_iter()
        .map(|meta| meta.slug)
        .collect();
    assert_eq!(backlinks, vec!["a"]);
    update_view(&app, cx, |view, _, cx| view.toggle_backlinks(cx));
    app.draw(cx);

    update_view(&app, cx, |view, _, cx| view.open_backlink("a".into(), cx));
    assert_eq!(app.active_note(cx).as_deref(), Some("a"));
    assert!(header(&app, cx).backlinks.is_empty());
}

#[gpui::test]
fn a_template_header_edits_icon_tags_and_name(cx: &mut TestAppContext) {
    let dir = template_cave();
    let app = App::with_cave(dir.path(), cx);
    open_template(&app, "day", cx);
    app.draw(cx);
    let shown = header(&app, cx);
    assert_eq!(shown.icon.as_deref(), Some("Star"));
    assert_eq!(shown.tags, vec!["day"]);
    assert!(!shown.favorite && shown.backlinks.is_empty());
    type_at_end(&app, "x", cx);

    update_view(&app, cx, |view, _, cx| {
        view.choose_icon(Some("Zap".into()), cx)
    });
    update_view(&app, cx, |view, window, cx| {
        view.type_tag("Daily", true, window, cx)
    });

    let raw = read(dir.path(), ".granit/templates/day.md");
    assert!(
        raw.starts_with("---\nicon: Zap\n# keep me\ntags: [day, daily]\n"),
        "only the touched keys changed: {raw}"
    );
    assert!(raw.ends_with(&format!("---\n{TEMPLATE_BODY}x")), "{raw}");
    assert_eq!(read(dir.path(), "day.md"), "# The note\n");
    let listed = cx.update(|cx| cave_ops::list_templates(cx)).unwrap();
    assert_eq!(listed[0].icon.as_deref(), Some("Zap"), "the panel's icon");

    update_view(&app, cx, |view, window, cx| {
        view.start_title_edit(window, cx);
        view.type_title("week", window, cx);
        view.commit_title(cx);
    });
    let shown = cx.read_entity(&app.workspace, |workspace, cx| {
        workspace.open_template_slug(cx)
    });
    assert_eq!(shown.as_deref(), Some("week"));
    assert!(dir.path().join(".granit/templates/week.md").exists());
    assert_eq!(text(&app, cx), format!("{TEMPLATE_BODY}x"));
    app.draw(cx);
}

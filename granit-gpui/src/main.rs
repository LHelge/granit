//! GPUI frontend spike: opens the cave given on the command line, shows its
//! note tree in the sidebar and the selected note in a live-preview editor
//! that saves back through the cave.

mod link_nav;
mod note_editor;
mod roundtrip;
mod save;
mod sidebar;
mod theme;
mod tree_model;
mod wiki_links;

use std::{path::PathBuf, rc::Rc};

use gpui::{
    div, px, size, App, AppContext, Bounds, Context, Entity, Focusable, InteractiveElement,
    IntoElement, KeyBinding, MouseButton, MouseDownEvent, ParentElement, Pixels, Point, Render,
    ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Subscription, Task,
    TitlebarOptions, Window, WindowBounds, WindowOptions,
};
use gpui_kit::component::{
    tree::{tree, TreeState},
    ActiveTheme,
};
use granit_lib::cave::{Cave, CaveError};
use zorite_editor::{EditorEvent, EditorState};

use link_nav::LinkCompletion;
use note_editor::CodeHighlighter;
use save::{FlushOutcome, NoteSession, SaveStatus, AUTOSAVE_DELAY};
use tree_model::{build_tree, item_kind, note_item_id, to_tree_items, ItemKind, TreeNode};
use wiki_links::LinkIndex;

gpui_kit::actions!(granit, [Quit, Save]);

#[derive(Debug, thiserror::Error)]
enum OpenError {
    #[error("No cave path given. Usage: granit-gpui <cave-path>")]
    MissingPath,
    #[error(transparent)]
    Cave(#[from] CaveError),
}

/// What loading a cave produces: the cave, its display tree and its link
/// targets.
type LoadedCave = (Cave, Vec<TreeNode>, LinkIndex);

/// Open and scan the cave, then build its display tree and link index.
/// Blocking file I/O; runs on the background executor.
fn load_cave(path: PathBuf) -> Result<LoadedCave, OpenError> {
    let cave = Cave::open(path)?;
    let notes = cave.list_notes()?;
    let slugs = notes.iter().map(|meta| meta.slug.clone()).collect();
    let tree = build_tree(notes, cave.list_folders()?);
    let links = LinkIndex::from_cave(&cave, slugs);
    Ok((cave, tree, links))
}

enum CaveStatus {
    Loading,
    Failed(SharedString),
    Open {
        cave: Box<Cave>,
        tree: Entity<TreeState>,
        /// Link targets for completion and broken-link marking; rebuilt
        /// after saves, which can change anchors and broken targets.
        links: LinkIndex,
        _selection: Subscription,
    },
}

/// The note shown in the main area.
enum OpenNote {
    None,
    Loaded(NoteSession),
    Failed {
        slug: SharedString,
        message: SharedString,
    },
}

struct Workspace {
    status: CaveStatus,
    note: OpenNote,
    /// The open note's editor. Every opened note gets a fresh one: the
    /// editor maps mouse positions through the layout of its last paint,
    /// and `set_text` leaves that layout in place, so re-using one editor
    /// for a shorter note panics on input that arrives before the next
    /// paint.
    editor: Entity<EditorState>,
    /// Code-block highlighting shared by every editor this workspace
    /// creates.
    highlighter: Rc<CodeHighlighter>,
    /// Scroll container around the editor, which grows to its content height.
    scroll: ScrollHandle,
    /// Pending debounced save; replacing or dropping it cancels the timer.
    autosave: Option<Task<()>>,
    /// The open `[[` completion popup, if any.
    completion: Option<LinkCompletion>,
    /// Start offset of a `[[` whose popup was dismissed with Escape; it
    /// stays closed until the caret leaves that link.
    dismissed_link: Option<usize>,
    /// Window position of the last mouse press in the editor area, used to
    /// tell which construct a link click came from.
    last_press: Option<Point<Pixels>>,
    /// Heading offset to scroll to once the editor has painted it.
    pending_anchor: Option<usize>,
    editor_events: Subscription,
    _appearance: Subscription,
    _quit: Subscription,
}

impl Workspace {
    fn new(cave_path: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let highlighter = CodeHighlighter::new(cx.theme().highlight_theme.clone());
        let (editor, editor_events) = Self::new_editor(&highlighter, window, cx);

        let appearance = cx.observe_window_appearance(window, |this, window, cx| {
            theme::sync(window, cx);
            Self::apply_theme(&this.editor, &this.highlighter, cx);
        });

        // Write pending edits before the window closes; a failed save keeps
        // the window open so the error stays visible.
        let this = cx.entity().downgrade();
        window.on_window_should_close(cx, move |_, cx| {
            this.update(cx, |this, cx| {
                !matches!(this.flush(cx), FlushOutcome::Failed(_))
            })
            .unwrap_or(true)
        });
        let quit = cx.on_app_quit(|this, cx| {
            this.flush(cx);
            async {}
        });

        let status = match cave_path {
            Some(path) => {
                cx.spawn_in(window, async move |this, cx| {
                    let result = cx.background_spawn(async move { load_cave(path) }).await;
                    this.update_in(cx, |this, window, cx| {
                        this.finish_loading(result, window, cx)
                    })
                    .ok();
                })
                .detach();
                CaveStatus::Loading
            }
            None => CaveStatus::Failed(OpenError::MissingPath.to_string().into()),
        };
        Self {
            status,
            note: OpenNote::None,
            editor,
            highlighter,
            scroll: ScrollHandle::new(),
            autosave: None,
            completion: None,
            dismissed_link: None,
            last_press: None,
            pending_anchor: None,
            editor_events,
            _appearance: appearance,
            _quit: quit,
        }
    }

    /// A styled, empty editor and the subscription that routes its events
    /// to the workspace.
    fn new_editor(
        highlighter: &Rc<CodeHighlighter>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (Entity<EditorState>, Subscription) {
        let editor = cx.new(|cx| EditorState::new(window, cx).with_placeholder("Empty note"));
        Self::apply_theme(&editor, highlighter, cx);
        let events = cx.subscribe_in(&editor, window, Self::on_editor_event);
        (editor, events)
    }

    /// Style the editor from the active theme: live-preview colors and fonts
    /// plus code-block token colors.
    pub(crate) fn apply_theme(
        editor: &Entity<EditorState>,
        highlighter: &Rc<CodeHighlighter>,
        cx: &mut App,
    ) {
        let style = note_editor::markdown_style(cx.theme());
        highlighter.set_theme(cx.theme().highlight_theme.clone());
        let hook = highlighter.hook();
        editor.update(cx, |editor, cx| {
            editor.set_code_highlighter(hook);
            editor.set_markdown_style(style, cx);
        });
    }

    fn finish_loading(
        &mut self,
        result: Result<LoadedCave, OpenError>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.status = match result {
            Ok((cave, nodes, links)) => {
                let items = to_tree_items(&nodes);
                let tree = cx.new(|cx| TreeState::new(cx).items(items));
                let selection = cx.observe_in(&tree, window, Self::sync_selection);
                CaveStatus::Open {
                    cave: Box::new(cave),
                    tree,
                    links,
                    _selection: selection,
                }
            }
            Err(err) => CaveStatus::Failed(format!("Could not open cave: {err}").into()),
        };
        // `GRANIT_OPEN=<slug>` opens a note at startup, for measurements.
        if let Ok(slug) = std::env::var("GRANIT_OPEN") {
            self.open_note(&slug, window, cx);
            self.select_in_tree(&slug, cx);
        }
        cx.notify();
    }

    /// Follow the tree selection when it lands on a note. Selecting a folder
    /// keeps the current note open.
    fn sync_selection(
        &mut self,
        tree: Entity<TreeState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(item) = tree.read(cx).selected_item() else {
            return;
        };
        let Some(ItemKind::Note(slug)) = item_kind(&item.id) else {
            return;
        };
        let slug = slug.to_string();
        self.open_note(&slug, window, cx);
    }

    fn open_slug(&self) -> Option<&str> {
        match &self.note {
            OpenNote::None => None,
            OpenNote::Loaded(session) => Some(session.slug()),
            OpenNote::Failed { slug, .. } => Some(slug),
        }
    }

    /// Show `slug` in a fresh editor, writing the current note's pending
    /// edits first. If that write fails, the current note stays open and the
    /// tree selection is moved back to it. The new editor takes focus when
    /// the old one had it.
    fn open_note(&mut self, slug: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.open_slug() == Some(slug) {
            return;
        }
        if let FlushOutcome::Failed(_) = self.flush(cx) {
            self.reselect_open_note(cx);
            return;
        }
        self.autosave = None;
        self.completion = None;
        self.dismissed_link = None;
        self.pending_anchor = None;

        let CaveStatus::Open { cave, .. } = &self.status else {
            return;
        };
        self.note = match cave.read_note(slug) {
            Ok(doc) => {
                let had_focus = self.editor.read(cx).focus_handle(cx).is_focused(window);
                let (editor, events) = Self::new_editor(&self.highlighter, window, cx);
                editor.update(cx, |editor, cx| editor.set_text(doc.content.clone(), cx));
                if had_focus {
                    editor.update(cx, |editor, cx| editor.focus(window, cx));
                }
                self.editor = editor;
                self.editor_events = events;
                self.scroll.set_offset(gpui::point(px(0.), px(0.)));
                OpenNote::Loaded(NoteSession::new(slug, doc.content))
            }
            Err(err) => OpenNote::Failed {
                slug: slug.to_string().into(),
                message: format!("Could not open {slug}: {err}").into(),
            },
        };
        // Loading keeps the previous note's diagnostics; replace them.
        self.mark_broken_links(cx);
        cx.notify();
    }

    /// Point the tree selection back at the open note.
    fn reselect_open_note(&mut self, cx: &mut Context<Self>) {
        let Some(slug) = self.open_slug().map(str::to_string) else {
            return;
        };
        self.select_in_tree(&slug, cx);
    }

    /// Select `slug` in the sidebar tree, expanding its folders and
    /// scrolling it into view.
    fn select_in_tree(&mut self, slug: &str, cx: &mut Context<Self>) {
        let CaveStatus::Open { tree, .. } = &self.status else {
            return;
        };
        let id = SharedString::from(note_item_id(slug));
        tree.update(cx, |tree, cx| {
            tree.reveal_item(&id, gpui::ScrollStrategy::Center, cx);
            if let Some(ix) = tree.index_of(&id) {
                tree.set_selected_index(Some(ix), cx);
            }
        });
        cx.notify();
    }

    /// Write the open note if it has unsaved edits.
    fn flush(&mut self, cx: &mut Context<Self>) -> FlushOutcome {
        let OpenNote::Loaded(session) = &mut self.note else {
            return FlushOutcome::Unchanged;
        };
        let CaveStatus::Open { cave, links, .. } = &mut self.status else {
            return FlushOutcome::Unchanged;
        };
        let text = self.editor.read(cx).text();
        let outcome = session.flush(text, |slug, text| cave.save_note(slug, text).map(|_| ()));
        match &outcome {
            FlushOutcome::Failed(err) => {
                eprintln!("granit-gpui: could not save {}: {err}", session.slug());
            }
            // The save may have added or removed anchors or link targets.
            FlushOutcome::Saved => *links = LinkIndex::from_cave(cave, links.slugs().to_vec()),
            FlushOutcome::Unchanged => {}
        }
        if outcome == FlushOutcome::Saved {
            self.mark_broken_links(cx);
        }
        cx.notify();
        outcome
    }

    fn save(&mut self, _: &Save, _: &mut Window, cx: &mut Context<Self>) {
        self.autosave = None;
        self.flush(cx);
    }

    fn on_editor_event(
        &mut self,
        emitter: &Entity<EditorState>,
        event: &EditorEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Only the open note's editor speaks for the workspace.
        if *emitter != self.editor {
            return;
        }
        match event {
            EditorEvent::Changed => {
                self.note_edited(window, cx);
                self.mark_broken_links(cx);
                self.update_completion(true, window, cx);
            }
            EditorEvent::SelectionChanged => {
                self.update_completion(false, window, cx);
                self.follow_caret(window, cx);
            }
            EditorEvent::OpenWikiLink(target) => self.follow_wiki_link(target, window, cx),
            EditorEvent::OpenLink(url) => self.open_external_link(url, cx),
            _ => {}
        }
    }

    /// Record a user edit of the open note: start the debounced save and
    /// keep the caret in view.
    fn note_edited(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let OpenNote::Loaded(session) = &mut self.note else {
            return;
        };
        let generation = session.mark_edited();
        self.autosave = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(AUTOSAVE_DELAY).await;
            this.update(cx, |this, cx| {
                let latest = matches!(
                    &this.note,
                    OpenNote::Loaded(session) if session.is_latest_edit(generation)
                );
                if latest {
                    this.autosave = None;
                    this.flush(cx);
                }
            })
            .ok();
        }));
        cx.notify();
        self.follow_caret(window, cx);
    }

    /// After the next paint (when the editor's layout reflects the change),
    /// scroll the caret into view.
    fn follow_caret(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.on_next_frame(window, |this, _, cx| {
            let Some(caret) = this.editor.read(cx).caret_screen_bounds() else {
                return;
            };
            if note_editor::scroll_caret_into_view(&this.scroll, caret.top(), caret.bottom()) {
                cx.notify();
            }
        });
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let content = match &self.status {
            CaveStatus::Open {
                cave, tree: state, ..
            } => {
                let name = cave
                    .path()
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| cave.path().display().to_string());
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(name),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .px_1()
                            .child(tree(state, sidebar::render_entry)),
                    )
                    .into_any_element()
            }
            CaveStatus::Loading | CaveStatus::Failed(_) => div().into_any_element(),
        };
        div()
            .w(px(260.))
            .h_full()
            .flex_none()
            .overflow_hidden()
            .bg(theme.sidebar)
            .text_color(theme.sidebar_foreground)
            .border_r_1()
            .border_color(theme.sidebar_border)
            .child(content)
    }

    /// A centered message in the main area.
    fn render_message(message: SharedString, color: gpui::Hsla) -> gpui::AnyElement {
        div()
            .flex_1()
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .p_6()
            .text_color(color)
            .child(message)
            .into_any_element()
    }

    /// Title bar above the editor: the note's slug and its save state.
    fn render_note_header(
        &self,
        session: &NoteSession,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let (label, color): (SharedString, _) = match session.status() {
            SaveStatus::Clean => ("".into(), theme.muted_foreground),
            SaveStatus::Edited => ("Edited".into(), theme.muted_foreground),
            SaveStatus::Failed(err) => (format!("Save failed: {err}").into(), theme.danger),
        };
        div()
            .flex_none()
            .flex()
            .flex_row()
            .items_center()
            .gap_3()
            .px_6()
            .py_2()
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(SharedString::from(session.slug().to_string())),
            )
            .child(div().flex_none().text_sm().text_color(color).child(label))
    }

    fn render_main(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme();
        let session = match (&self.status, &self.note) {
            (CaveStatus::Failed(err), _) => {
                return Self::render_message(err.clone(), theme.danger);
            }
            (CaveStatus::Loading, _) => {
                let color = theme.muted_foreground;
                return Self::render_message("Opening cave…".into(), color);
            }
            (CaveStatus::Open { .. }, OpenNote::None) => {
                let color = theme.muted_foreground;
                return Self::render_message("No note open".into(), color);
            }
            (CaveStatus::Open { .. }, OpenNote::Failed { message, .. }) => {
                let color = theme.danger;
                return Self::render_message(message.clone(), color);
            }
            (CaveStatus::Open { .. }, OpenNote::Loaded(session)) => session,
        };

        let font_family = theme.font_family.clone();
        let font_size = theme.font_size;
        let foreground = theme.foreground;
        let editor = self.editor.clone();
        // The editor takes its font and text color from the ambient text
        // style.
        let editor_area = div()
            .min_h_full()
            .px_8()
            .py_6()
            .font_family(font_family)
            .text_size(font_size)
            .text_color(foreground)
            // Runs before the editor's own press handling, so a link click
            // can be traced back to the text under the pointer.
            .capture_any_mouse_down(cx.listener(|this, event: &MouseDownEvent, _, _| {
                this.last_press = Some(event.position);
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    let handle = this.editor.read(cx).focus_handle(cx);
                    if !handle.is_focused(window) {
                        window.focus(&handle, cx);
                    }
                }),
            )
            .child(div().max_w(px(820.)).child(editor));
        let editor_area = self.intercept_completion_keys(editor_area, cx);
        let completion = self.render_completion(cx);
        div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .child(self.render_note_header(session, cx))
            .child(
                div()
                    .id("note-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .child(editor_area),
            )
            .children(completion)
            .into_any_element()
    }
}

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .size_full()
            .flex()
            .flex_row()
            .bg(theme.background)
            .text_color(theme.foreground)
            .on_action(cx.listener(Self::save))
            .child(self.render_sidebar(cx))
            .child(self.render_main(cx))
    }
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    let first = args.next();
    // Hidden mode for gate G2: `--roundtrip <cave> [...]` (see `roundtrip`).
    if first.as_deref() == Some(std::ffi::OsStr::new("--roundtrip")) {
        roundtrip::run_cli(args.collect());
    }
    let cave_path = first.map(PathBuf::from);

    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx: &mut App| {
            gpui_kit::init(cx);
            zorite_editor::bind_keys(cx);

            cx.bind_keys([
                KeyBinding::new("cmd-q", Quit, None),
                KeyBinding::new("cmd-s", Save, None),
            ]);
            cx.on_action(|_: &Quit, cx: &mut App| cx.quit());
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1100.), px(720.)),
                    cx,
                ))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Granit".into()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |window, cx| {
                theme::install(window, cx);
                cx.new(|cx| Workspace::new(cave_path, window, cx))
            })
            .expect("failed to open window");
            cx.activate(true);
        });
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use gpui::{AppContext, EntityInputHandler, Modifiers, TestAppContext, WindowHandle};

    use super::*;

    const NOTE_A: &str = "---\ntags:\n  - tag\ncreated_at: \"2026-01-01T00:00:00Z\"\n\
                          modified_at: \"2026-01-01T00:00:00Z\"\n---\n# A\n\nAlpha\n";
    const NOTE_B: &str = "# B\n";

    pub(crate) fn fixture_cave() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".granit")).unwrap();
        std::fs::write(dir.path().join("a.md"), NOTE_A).unwrap();
        std::fs::create_dir_all(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/b.md"), NOTE_B).unwrap();
        dir
    }

    pub(crate) fn open_workspace(path: &Path, cx: &mut TestAppContext) -> WindowHandle<Workspace> {
        let path = path.to_path_buf();
        let window = cx.update(|cx| {
            gpui_kit::init(cx);
            cx.open_window(Default::default(), |window, cx| {
                theme::install(window, cx);
                cx.new(|cx| Workspace::new(Some(path), window, cx))
            })
            .unwrap()
        });
        cx.run_until_parked();
        window
    }

    pub(crate) fn open(window: WindowHandle<Workspace>, slug: &str, cx: &mut TestAppContext) {
        window
            .update(cx, |this, window, cx| this.open_note(slug, window, cx))
            .unwrap();
        cx.run_until_parked();
    }

    /// Append `text` at the end of the document as typed input.
    fn type_at_end(window: WindowHandle<Workspace>, text: &str, cx: &mut TestAppContext) {
        window
            .update(cx, |this, window, cx| {
                this.editor.update(cx, |editor, cx| {
                    let end = editor.text().len();
                    editor.set_cursor(end, cx);
                    editor.replace_text_in_range(None, text, window, cx);
                });
            })
            .unwrap();
        cx.run_until_parked();
    }

    pub(crate) fn editor_text(window: WindowHandle<Workspace>, cx: &mut TestAppContext) -> String {
        window
            .update(cx, |this, _, cx| this.editor.read(cx).text().to_string())
            .unwrap()
    }

    fn body_of(raw: &str) -> &str {
        raw.rsplit_once("---\n").map_or(raw, |(_, body)| body)
    }

    #[gpui::test]
    fn opening_and_switching_without_edits_writes_nothing(cx: &mut TestAppContext) {
        let dir = fixture_cave();
        let window = open_workspace(dir.path(), cx);
        cx.update(|cx| {
            let name = &cx.theme().highlight_theme.name;
            assert!(name.starts_with("Catppuccin"), "{name}");
        });
        open(window, "a", cx);
        assert_eq!(editor_text(window, cx), "# A\n\nAlpha\n");
        open(window, "b", cx);
        assert_eq!(editor_text(window, cx), NOTE_B);
        open(window, "a", cx);
        cx.executor().advance_clock(AUTOSAVE_DELAY * 2);
        cx.run_until_parked();

        assert_eq!(
            std::fs::read_to_string(dir.path().join("a.md")).unwrap(),
            NOTE_A
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("sub/b.md")).unwrap(),
            NOTE_B
        );
    }

    #[gpui::test]
    fn save_action_writes_the_body_and_keeps_frontmatter(cx: &mut TestAppContext) {
        let dir = fixture_cave();
        let window = open_workspace(dir.path(), cx);
        open(window, "a", cx);
        type_at_end(window, "!", cx);
        window
            .update(cx, |this, window, cx| this.save(&Save, window, cx))
            .unwrap();

        let raw = std::fs::read_to_string(dir.path().join("a.md")).unwrap();
        assert_eq!(body_of(&raw), "# A\n\nAlpha\n!");
        assert!(raw.starts_with("---\ntags:\n  - tag\ncreated_at: \"2026-01-01T00:00:00Z\"\n"));
    }

    #[gpui::test]
    fn edits_are_saved_after_the_debounce(cx: &mut TestAppContext) {
        let dir = fixture_cave();
        let window = open_workspace(dir.path(), cx);
        open(window, "b", cx);
        type_at_end(window, "x", cx);
        cx.executor().advance_clock(AUTOSAVE_DELAY / 2);
        cx.run_until_parked();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("sub/b.md")).unwrap(),
            NOTE_B
        );

        // A further edit restarts the quiet period.
        type_at_end(window, "y", cx);
        cx.executor().advance_clock(AUTOSAVE_DELAY * 3 / 4);
        cx.run_until_parked();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("sub/b.md")).unwrap(),
            NOTE_B
        );

        cx.executor().advance_clock(AUTOSAVE_DELAY);
        cx.run_until_parked();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("sub/b.md")).unwrap(),
            "# B\nxy"
        );
    }

    #[gpui::test]
    fn switching_notes_flushes_pending_edits(cx: &mut TestAppContext) {
        let dir = fixture_cave();
        let window = open_workspace(dir.path(), cx);
        open(window, "b", cx);
        type_at_end(window, "z", cx);
        open(window, "a", cx);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("sub/b.md")).unwrap(),
            "# B\nz"
        );
        assert_eq!(editor_text(window, cx), "# A\n\nAlpha\n");
        // The cancelled debounce does not write the new note.
        cx.executor().advance_clock(AUTOSAVE_DELAY * 2);
        cx.run_until_parked();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("a.md")).unwrap(),
            NOTE_A
        );
    }

    #[gpui::test]
    fn failed_save_is_shown_and_keeps_the_note_open(cx: &mut TestAppContext) {
        let dir = fixture_cave();
        let window = open_workspace(dir.path(), cx);
        open(window, "b", cx);
        type_at_end(window, "z", cx);
        // Make the write fail: the note's folder is replaced by a file.
        std::fs::remove_dir_all(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub"), "").unwrap();

        open(window, "a", cx);
        window
            .update(cx, |this, _, _| {
                let OpenNote::Loaded(session) = &this.note else {
                    panic!("note b should stay open");
                };
                assert_eq!(session.slug(), "b");
                assert!(matches!(session.status(), SaveStatus::Failed(_)));
            })
            .unwrap();
        assert_eq!(editor_text(window, cx), "# B\nz");
    }

    #[gpui::test]
    fn rich_note_paints_headlessly(cx: &mut TestAppContext) {
        let dir = fixture_cave();
        let body = "# Title {#title}\n\n**bold** [[a]] [[a|label]] #tag\n\n\
                    - [ ] open\n- [x] done\n\n| a | b |\n| - | - |\n| 1 | 2 |\n\n\
                    ```rust\nfn main() {}\n```\n\n> [!NOTE]\n> alert\n";
        std::fs::write(dir.path().join("rich.md"), body).unwrap();
        let window = open_workspace(dir.path(), cx);
        open(window, "rich", cx);
        cx.update_window(window.into(), |_, window, cx| {
            let _ = window.draw(cx);
        })
        .unwrap();
        let caret = window
            .update(cx, |this, _, cx| this.editor.read(cx).caret_screen_bounds())
            .unwrap();
        assert!(caret.is_some(), "the editor laid out its text");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("rich.md")).unwrap(),
            body
        );
    }

    fn draw(window: WindowHandle<Workspace>, cx: &mut TestAppContext) {
        cx.update_window(window.into(), |_, window, cx| {
            let _ = window.draw(cx);
        })
        .unwrap();
    }

    /// Window position of the painted text at byte `offset` of the open note.
    fn painted_position(
        window: WindowHandle<Workspace>,
        offset: usize,
        cx: &mut TestAppContext,
    ) -> Point<Pixels> {
        let caret = window
            .update(cx, |this, _, cx| {
                this.editor.read(cx).bounds_for_offset(offset)
            })
            .unwrap()
            .expect("the offset was painted");
        gpui::point(caret.left() + px(2.), caret.top() + caret.size.height / 2.)
    }

    /// A cave with a note of many short lines next to the two-line `b`.
    fn long_cave() -> (tempfile::TempDir, String) {
        let dir = fixture_cave();
        let long: String = (0..60).map(|i| format!("Line {i}\n")).collect();
        std::fs::write(dir.path().join("long.md"), &long).unwrap();
        (dir, long)
    }

    /// Run `act` on the workspace, then deliver a left press at `position`
    /// before anything paints again: on macOS, input queued behind a slow
    /// frame reaches the views in that state.
    fn press_before_paint(
        window: WindowHandle<Workspace>,
        position: Point<Pixels>,
        act: impl FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>),
        cx: &mut TestAppContext,
    ) {
        let workspace = window.root(cx).unwrap();
        cx.update_window(window.into(), |_, window, cx| {
            workspace.update(cx, |this, cx| act(this, window, cx));
            let _ = window.dispatch_event(
                gpui::PlatformInput::MouseDown(MouseDownEvent {
                    button: MouseButton::Left,
                    position,
                    modifiers: Modifiers::none(),
                    click_count: 1,
                    first_mouse: false,
                }),
                cx,
            );
            let _ = window.dispatch_event(
                gpui::PlatformInput::MouseUp(gpui::MouseUpEvent {
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

    /// A press over the previous note's lower rows, arriving after a switch
    /// to a shorter note and before the next paint.
    #[gpui::test]
    fn press_before_the_switched_note_is_painted_does_not_panic(cx: &mut TestAppContext) {
        let (dir, long) = long_cave();
        let window = open_workspace(dir.path(), cx);
        open(window, "long", cx);
        draw(window, cx);
        let position = painted_position(window, long.find("Line 20").unwrap(), cx);

        press_before_paint(
            window,
            position,
            |this, window, cx| this.open_note("b", window, cx),
            cx,
        );
        assert_eq!(editor_text(window, cx), NOTE_B);
        draw(window, cx);
    }

    /// The same after following a wiki-link, which switches notes from
    /// inside the editor's own press handling. The new editor keeps focus.
    #[gpui::test]
    fn press_after_following_a_link_does_not_panic_and_keeps_focus(cx: &mut TestAppContext) {
        let (dir, long) = long_cave();
        std::fs::write(dir.path().join("long.md"), format!("{long}[[b]]\n")).unwrap();
        let window = open_workspace(dir.path(), cx);
        open(window, "long", cx);
        window
            .update(cx, |this, window, cx| {
                this.editor
                    .update(cx, |editor, cx| editor.focus(window, cx))
            })
            .unwrap();
        draw(window, cx);
        let position = painted_position(window, long.find("Line 40").unwrap(), cx);

        press_before_paint(
            window,
            position,
            |this, window, cx| this.navigate_to_link("b", window, cx),
            cx,
        );
        assert_eq!(editor_text(window, cx), NOTE_B);
        draw(window, cx);
        let focused = window
            .update(cx, |this, window, cx| {
                this.editor.read(cx).focus_handle(cx).is_focused(window)
            })
            .unwrap();
        assert!(focused, "the new editor has focus");
    }

    /// Recorded editor behaviour (zorite-editor 0.11.7), not ours: within
    /// one editor, an edit that removes lines leaves the painted layout
    /// behind until the next paint, and a press over a removed row in that
    /// window indexes past the line table. Fails once the crate fixes it.
    #[gpui::test]
    #[should_panic(expected = "index out of bounds")]
    fn press_after_a_typed_edit_before_paint_panics_in_the_editor(cx: &mut TestAppContext) {
        let (dir, long) = long_cave();
        let window = open_workspace(dir.path(), cx);
        open(window, "long", cx);
        draw(window, cx);
        let position = painted_position(window, long.find("Line 20").unwrap(), cx);

        // Select all and type over it, as the user would.
        press_before_paint(
            window,
            position,
            |this, window, cx| {
                this.editor.update(cx, |editor, cx| {
                    let utf16_len = editor.text().encode_utf16().count();
                    editor.replace_text_in_range(Some(0..utf16_len), "x", window, cx);
                })
            },
            cx,
        );
    }

    /// The same for a host edit through `replace_range`, which is how an
    /// accepted `[[` completion is applied.
    #[gpui::test]
    #[should_panic(expected = "index out of bounds")]
    fn press_after_replace_range_before_paint_panics_in_the_editor(cx: &mut TestAppContext) {
        let (dir, long) = long_cave();
        let window = open_workspace(dir.path(), cx);
        open(window, "long", cx);
        draw(window, cx);
        let position = painted_position(window, long.find("Line 20").unwrap(), cx);

        press_before_paint(
            window,
            position,
            |this, _, cx| {
                this.editor.update(cx, |editor, cx| {
                    let len = editor.text().len();
                    editor.replace_range(0..len, "[[b]]", cx);
                })
            },
            cx,
        );
    }

    /// Recorded editor behaviour: when the edit keeps the line count but
    /// shortens the lines, the stale press does not panic itself. It parks
    /// the caret past the end of the text, and the next keystroke panics
    /// slicing the document.
    #[gpui::test]
    #[should_panic(expected = "out of bounds for string")]
    fn press_after_shortening_lines_parks_the_caret_out_of_range(cx: &mut TestAppContext) {
        let dir = fixture_cave();
        let long: String = (0..30)
            .map(|i| format!("Line {i} with a long tail of words\n"))
            .collect();
        std::fs::write(dir.path().join("long.md"), &long).unwrap();
        let window = open_workspace(dir.path(), cx);
        open(window, "long", cx);
        draw(window, cx);
        let position = painted_position(window, long.rfind("words").unwrap(), cx);

        press_before_paint(
            window,
            position,
            |this, _, cx| {
                this.editor.update(cx, |editor, cx| {
                    let len = editor.text().len();
                    editor.replace_range(0..len, &"x\n".repeat(30), cx);
                })
            },
            cx,
        );
        let (cursor, len) = window
            .update(cx, |this, _, cx| {
                let editor = this.editor.read(cx);
                (editor.cursor(), editor.text().len())
            })
            .unwrap();
        assert!(cursor > len, "caret {cursor} past the end ({len})");
        window
            .update(cx, |this, window, cx| {
                this.editor.update(cx, |editor, cx| {
                    editor.replace_text_in_range(None, "z", window, cx)
                })
            })
            .unwrap();
    }

    #[gpui::test]
    fn empty_note_opens_and_saves(cx: &mut TestAppContext) {
        let dir = fixture_cave();
        std::fs::write(dir.path().join("empty.md"), "").unwrap();
        let window = open_workspace(dir.path(), cx);
        open(window, "empty", cx);
        assert_eq!(editor_text(window, cx), "");
        type_at_end(window, "hi", cx);
        open(window, "a", cx);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("empty.md")).unwrap(),
            "hi"
        );
    }
}

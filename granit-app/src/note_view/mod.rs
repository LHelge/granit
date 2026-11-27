//! The main area: the open note in the live-preview editor (`zorite-editor`).
//!
//! The workspace drives the view with [`NoteView::open`] (which writes the
//! current note's pending edits first), [`NoteView::reveal`] (scroll to a
//! heading anchor, a line or an offset), [`NoteView::reload`] after a cave
//! change, and [`NoteView::flush`] before the cave is replaced. The view asks
//! the workspace to follow a wiki-link with [`NoteViewEvent::OpenNote`].
//!
//! Saving: an edit starts a [`save::AUTOSAVE_DELAY`] holdoff; the save key
//! writes at once, and so do a note switch, the editor losing focus, a cave
//! switch, closing the window and quitting. Every write goes through a
//! [`save::SaveSnapshot`] of (slug, text) taken at that moment, and an
//! unedited note is never written.
//!
//! Each opened note gets a fresh [`EditorState`]: the editor maps mouse
//! positions through the layout of its last paint, so a reused editor could
//! act on stale rows after loading a shorter note. One [`CodeHighlighter`]
//! and one [`ImageCache`] are shared by all of them.
//!
//! Submodules: [`save`] (save bookkeeping), [`wiki_links`] (link parsing and
//! completion ranking), [`links`] (navigation and the `[[` popup), [`find`]
//! (the find bar), [`images`], [`style`] and [`commands`] (link and task
//! shortcuts).

mod commands;
mod find;
mod images;
mod links;
mod save;
mod style;
mod wiki_links;

#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::rc::Rc;

use gpui::{
    div, px, AnyWindowHandle, App, AppContext as _, Context, Entity, EventEmitter, Focusable,
    FontWeight, InteractiveElement as _, IntoElement, MouseButton, MouseDownEvent, ParentElement,
    Pixels, Point, Render, ScrollHandle, SharedString, StatefulInteractiveElement as _, Styled,
    Subscription, Task, Window,
};
use gpui_kit::component::{ActiveTheme, Theme};
use log::warn;
use zorite_editor::{EditorEvent, EditorState};

use crate::cave_ops;
use crate::config_ops::{ConfigEvent, ConfigEvents};
use crate::events::{CaveEvent, CaveEvents};
use crate::host::Host;
use find::FindBar;
use images::{ImageBase, ImageCache};
use links::LinkCompletion;
pub use save::AUTOSAVE_DELAY;
use save::{FlushOutcome, NoteSession, SaveSnapshot, SaveStatus};
use style::CodeHighlighter;
use wiki_links::LinkIndex;

gpui_kit::actions!(
    note_view,
    [
        /// Write the open note now.
        SaveNote,
        /// Open (or focus) the find bar.
        FindInNote,
        /// Wrap the selection as a link, or insert `[[]]`.
        InsertLink,
        /// Toggle the task checkbox of the selected lines.
        ToggleTask,
    ]
);

/// The key context of the note view; its bindings apply while focus is in
/// the editor or the find bar.
pub const KEY_CONTEXT: &str = "NoteView";

// `KeyBinding::new` rejects GPUI's `secondary-` prefix, so the platform's
// command modifier is spelled out: `cmd-` on macOS, `ctrl-` elsewhere.

/// Save the open note now.
#[cfg(target_os = "macos")]
pub const SAVE_KEY: &str = "cmd-s";
#[cfg(not(target_os = "macos"))]
pub const SAVE_KEY: &str = "ctrl-s";

/// Find in the open note.
#[cfg(target_os = "macos")]
pub const FIND_KEY: &str = "cmd-f";
#[cfg(not(target_os = "macos"))]
pub const FIND_KEY: &str = "ctrl-f";

/// Insert a wiki-link (the web editor's `Mod-k`).
#[cfg(target_os = "macos")]
pub const INSERT_LINK_KEY: &str = "cmd-k";
#[cfg(not(target_os = "macos"))]
pub const INSERT_LINK_KEY: &str = "ctrl-k";

/// Toggle a task checkbox (the web editor's `Mod-l`).
#[cfg(target_os = "macos")]
pub const TOGGLE_TASK_KEY: &str = "cmd-l";
#[cfg(not(target_os = "macos"))]
pub const TOGGLE_TASK_KEY: &str = "ctrl-l";

/// Bind the editor's keys and the note view's shortcuts. Call once at
/// startup.
pub fn bind_keys(cx: &mut App) {
    zorite_editor::bind_keys(cx);
    let context = Some(KEY_CONTEXT);
    cx.bind_keys([
        gpui::KeyBinding::new(SAVE_KEY, SaveNote, context),
        gpui::KeyBinding::new(FIND_KEY, FindInNote, context),
        gpui::KeyBinding::new(INSERT_LINK_KEY, InsertLink, context),
        gpui::KeyBinding::new(TOGGLE_TASK_KEY, ToggleTask, context),
    ]);
}

/// Where to put the caret (and the viewport) in a note.
///
/// Links reveal anchors; `Line` and `Offset` are for the panels that open a
/// note at a match (search, todos).
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub enum Reveal {
    /// The heading that declares `{#id}`.
    Anchor(String),
    /// The start of a 0-based line.
    Line(usize),
    /// A byte offset into the body.
    Offset(usize),
}

impl Reveal {
    /// The byte offset in `text` this points at; `None` for an anchor the
    /// note does not declare.
    fn offset_in(&self, text: &str) -> Option<usize> {
        match self {
            Reveal::Anchor(id) => wiki_links::anchor_heading_offset(text, id),
            Reveal::Line(line) => Some(wiki_links::line_offset(text, *line)),
            Reveal::Offset(offset) => Some((*offset).min(text.len())),
        }
    }
}

/// What the note view asks of the workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoteViewEvent {
    /// Open `slug` (a followed wiki-link), then reveal `reveal` in it.
    OpenNote {
        slug: String,
        reveal: Option<Reveal>,
    },
}

/// The open note.
struct Loaded {
    session: NoteSession,
    /// The cave the note belongs to; a save is refused once another cave is
    /// open.
    cave: PathBuf,
    images: ImageBase,
}

/// What the main area shows.
enum Note {
    /// No note is open.
    Empty,
    Loaded(Loaded),
    /// The note could not be read.
    Failed {
        slug: String,
        message: SharedString,
    },
}

/// How many frames a reveal waits for the editor to lay out its target.
const REVEAL_ATTEMPTS: u8 = 4;

/// Space kept above a revealed row scrolled to the top of the viewport.
const REVEAL_MARGIN: Pixels = px(16.);

pub struct NoteView {
    window: AnyWindowHandle,
    note: Note,
    /// The open note's editor, created by [`Self::install_editor`] once the
    /// note is loaded.
    editor: Option<Entity<EditorState>>,
    /// The previous note's editor while the next one is being installed.
    retiring_editor: Option<Entity<EditorState>>,
    /// The loaded body, when the editor's oldest undo step is the text it
    /// normalized on load (see [`load_text`]): an undo from this text would
    /// apply that normalization, so it is refused.
    undo_floor: Option<String>,
    /// The editor's event and blur subscriptions.
    editor_subscriptions: Vec<Subscription>,
    /// Bumped by every load, so a deferred install of an older one is
    /// dropped.
    load_generation: u64,
    /// Where to put the caret once the editor is installed.
    pending_reveal: Option<Reveal>,
    /// Caret and scroll position to keep when the open note is reloaded.
    pending_restore: Option<(usize, Point<Pixels>)>,
    /// A byte offset to scroll to the top once the editor has painted it,
    /// and the frames left to wait for that.
    pending_scroll: Option<(usize, u8)>,
    highlighter: Rc<CodeHighlighter>,
    image_cache: Rc<ImageCache>,
    links: LinkIndex,
    /// Scroll container around the editor, which grows to its content
    /// height.
    scroll: ScrollHandle,
    /// The pending debounced save; replacing or dropping it cancels it.
    autosave: Option<Task<()>>,
    /// The open `[[` completion popup.
    completion: Option<LinkCompletion>,
    /// Start of a `[[` whose popup was dismissed with Escape; it stays
    /// closed until the caret leaves that link.
    dismissed_link: Option<usize>,
    /// Window position of the last press in the editor area, to tell which
    /// construct a link click came from.
    last_press: Option<Point<Pixels>>,
    find: Option<FindBar>,
    /// The font the editor area was last drawn with, for tests.
    #[cfg(test)]
    drawn_font: std::cell::Cell<Option<(SharedString, Pixels)>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<NoteViewEvent> for NoteView {}

impl NoteView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Write pending edits before the window closes; a failed save keeps
        // the window open so the error stays visible.
        let this = cx.weak_entity();
        window.on_window_should_close(cx, move |_, cx| {
            this.update(cx, |this, cx| {
                !matches!(this.flush(cx), FlushOutcome::Failed(_))
            })
            .unwrap_or(true)
        });
        let subscriptions = vec![
            cx.on_app_quit(|this, cx| {
                this.flush(cx);
                async {}
            }),
            // A theme switch (appearance or settings) restyles the editor.
            cx.observe_global::<Theme>(|this, cx| this.apply_theme(cx)),
            // A font setting changes the ambient text style the editor draws with.
            cx.subscribe(
                &ConfigEvents::entity(cx),
                |_, _, event: &ConfigEvent, cx| {
                    if event.fonts {
                        cx.notify();
                    }
                },
            ),
            cx.subscribe(&CaveEvents::entity(cx), |this, _, event, _| {
                if *event == CaveEvent::Opened {
                    this.image_cache.clear();
                }
            }),
        ];
        Self {
            window: window.window_handle(),
            note: Note::Empty,
            editor: None,
            retiring_editor: None,
            undo_floor: None,
            editor_subscriptions: Vec::new(),
            load_generation: 0,
            pending_reveal: None,
            pending_restore: None,
            pending_scroll: None,
            highlighter: CodeHighlighter::new(cx.theme().highlight_theme.clone()),
            image_cache: ImageCache::new(),
            links: LinkIndex::default(),
            scroll: ScrollHandle::new(),
            autosave: None,
            completion: None,
            dismissed_link: None,
            last_press: None,
            find: None,
            #[cfg(test)]
            drawn_font: Default::default(),
            _subscriptions: subscriptions,
        }
    }

    /// The slug of the shown note.
    pub fn slug(&self) -> Option<&str> {
        match &self.note {
            Note::Empty => None,
            Note::Loaded(loaded) => Some(loaded.session.slug()),
            Note::Failed { slug, .. } => Some(slug),
        }
    }

    /// The text of the open note: the editor's, or the loaded body before
    /// the editor is installed.
    pub fn text<'a>(&'a self, cx: &'a App) -> Option<&'a str> {
        let Note::Loaded(loaded) = &self.note else {
            return None;
        };
        Some(match &self.editor {
            Some(editor) => editor.read(cx).text(),
            None => loaded.session.saved(),
        })
    }

    /// The open note's editor.
    #[cfg(test)]
    pub fn editor(&self) -> Option<&Entity<EditorState>> {
        self.editor.as_ref()
    }

    /// The font family and size the editor area was last drawn with.
    #[cfg(test)]
    pub fn drawn_font(&self) -> Option<(SharedString, Pixels)> {
        let font = self.drawn_font.take();
        self.drawn_font.set(font.clone());
        font
    }

    /// The name of the code-highlight theme the editors were last styled with.
    #[cfg(test)]
    pub fn highlight_theme_name(&self) -> SharedString {
        self.highlighter.theme_name()
    }

    /// The open note's save state.
    #[cfg(test)]
    fn save_status(&self) -> Option<SaveStatus> {
        match &self.note {
            Note::Loaded(loaded) => Some(loaded.session.status().clone()),
            _ => None,
        }
    }

    /// Show the note `slug`, or nothing, writing the current note's pending
    /// edits first. Returns the slug shown afterwards: if those edits could
    /// not be written, the current note stays open (with the error shown)
    /// and its slug is returned instead.
    pub fn open(&mut self, slug: Option<&str>, cx: &mut Context<Self>) -> Option<String> {
        if self.slug() == slug {
            return slug.map(str::to_string);
        }
        if let FlushOutcome::Failed(_) = self.flush(cx) {
            return self.slug().map(str::to_string);
        }
        self.load(slug, cx);
        slug.map(str::to_string)
    }

    /// Close the open note without writing it (it was deleted).
    pub fn forget(&mut self, cx: &mut Context<Self>) {
        self.load(None, cx);
    }

    /// Put the caret at `reveal` in the open note and scroll it to the top.
    pub fn reveal(&mut self, reveal: Reveal, cx: &mut Context<Self>) {
        if !matches!(self.note, Note::Loaded(_)) {
            return;
        }
        self.pending_reveal = Some(reveal);
        if self.editor.is_some() {
            self.apply_reveal(cx);
        }
    }

    /// The note `from` was renamed to `to`: keep it open (and its pending
    /// edits) under the new name.
    pub fn note_renamed(&mut self, from: &str, to: &str, cx: &mut Context<Self>) {
        if let Note::Loaded(loaded) = &mut self.note {
            if loaded.session.slug() == from {
                loaded.session.retarget(to);
                cx.notify();
            }
        }
    }

    /// Re-read the open note after a cave change. Unsaved edits win: a note
    /// with pending edits keeps them (the next save writes them over the
    /// change); one without is reloaded when its body changed on disk.
    pub fn reload(&mut self, cx: &mut Context<Self>) {
        self.refresh_links(cx);
        self.follow_note_dir(cx);
        let Note::Loaded(loaded) = &self.note else {
            return;
        };
        if loaded.session.is_dirty() {
            return;
        }
        let slug = loaded.session.slug().to_string();
        let changed = match cave_ops::read_note(&slug, cx) {
            Ok(doc) => doc.content != loaded.session.saved(),
            // Gone or unreadable: the workspace decides what to show.
            Err(_) => false,
        };
        if changed {
            let caret = self.editor.as_ref().map(|editor| editor.read(cx).cursor());
            self.load(Some(&slug), cx);
            // Stay where the reader was.
            self.pending_restore = Some((caret.unwrap_or(0), self.scroll.offset()));
        }
    }

    /// Write the open note if it has unsaved edits.
    pub fn flush(&mut self, cx: &mut Context<Self>) -> FlushOutcome {
        self.autosave = None;
        let Note::Loaded(loaded) = &mut self.note else {
            return FlushOutcome::Unchanged;
        };
        let Some(editor) = &self.editor else {
            return FlushOutcome::Unchanged;
        };
        let Some(snapshot) = loaded.session.snapshot(editor.read(cx).text()) else {
            return FlushOutcome::Unchanged;
        };
        let cave = loaded.cave.clone();
        let result = write_snapshot(&cave, &snapshot, cx);
        let Note::Loaded(loaded) = &mut self.note else {
            return FlushOutcome::Unchanged;
        };
        let outcome = loaded.session.finish(&snapshot, result);
        if let FlushOutcome::Failed(err) = &outcome {
            warn!("Could not save {}: {err}", snapshot.slug);
        }
        cx.notify();
        outcome
    }

    /// The open note was moved (or its folder renamed or moved): resolve its
    /// relative images from its new directory.
    fn follow_note_dir(&mut self, cx: &mut Context<Self>) {
        let Note::Loaded(loaded) = &mut self.note else {
            return;
        };
        let slug = loaded.session.slug().to_string();
        let Ok(note_dir) = Host::core(cx).with_cave(|cave| cave.note_dir(&slug)) else {
            return;
        };
        if note_dir == loaded.images.note_dir {
            return;
        }
        loaded.images.note_dir = note_dir;
        let provider = self.image_cache.provider(loaded.images.clone());
        if let Some(editor) = &self.editor {
            editor.update(cx, |editor, cx| {
                editor.set_block_image_provider(provider);
                cx.notify();
            });
        }
        self.request_images(cx);
    }

    /// Write pending edits before the cave is switched; the error, naming the
    /// note, when they could not be written.
    pub fn save_before_leaving(&mut self, cx: &mut Context<Self>) -> Result<(), String> {
        match self.flush(cx) {
            FlushOutcome::Failed(err) => Err(format!(
                "the edits to {} could not be saved: {err}",
                self.slug().unwrap_or_default()
            )),
            FlushOutcome::Saved | FlushOutcome::Unchanged => Ok(()),
        }
    }

    /// Replace the shown note with `slug` (or nothing), dropping the current
    /// one without writing it. The editor is created once the window is
    /// free (see [`Self::install_editor`]).
    fn load(&mut self, slug: Option<&str>, cx: &mut Context<Self>) {
        self.autosave = None;
        self.completion = None;
        self.dismissed_link = None;
        self.pending_reveal = None;
        self.pending_restore = None;
        self.pending_scroll = None;
        self.last_press = None;
        self.undo_floor = None;
        if let Some(find) = &mut self.find {
            find.clear_matches();
        }
        // The old editor stops speaking for (and saving to) the view at
        // once; it is kept until the new one is installed, to hand over
        // focus.
        self.editor_subscriptions.clear();
        if let Some(editor) = self.editor.take() {
            self.retiring_editor = Some(editor);
        }
        self.load_generation += 1;
        self.note = match slug {
            None => Note::Empty,
            Some(slug) => match load_note(slug, cx) {
                Ok(loaded) => Note::Loaded(loaded),
                Err(err) => Note::Failed {
                    slug: slug.to_string(),
                    message: format!("Could not open {slug}: {err}").into(),
                },
            },
        };
        self.refresh_links(cx);
        if matches!(self.note, Note::Loaded(_)) {
            let generation = self.load_generation;
            let window = self.window;
            let this = cx.weak_entity();
            // `EditorState::new` needs the window, which is not at hand
            // here (and is borrowed while it dispatches the event that got
            // us here), so the editor is installed at the end of the effect
            // cycle, before anything paints.
            cx.defer(move |cx| {
                window
                    .update(cx, |_, window, cx| {
                        this.update(cx, |this, cx| {
                            if this.load_generation == generation {
                                this.install_editor(window, cx);
                            }
                        })
                    })
                    .ok();
            });
        } else {
            self.retiring_editor = None;
        }
        cx.notify();
    }

    /// Create the editor for the loaded note, taking over focus from the
    /// previous one.
    fn install_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Note::Loaded(loaded) = &self.note else {
            return;
        };
        let body = loaded.session.saved().to_string();
        let provider = self.image_cache.provider(loaded.images.clone());
        let had_focus = self
            .retiring_editor
            .take()
            .is_some_and(|editor| editor.read(cx).focus_handle(cx).is_focused(window));

        let editor = cx.new(|cx| EditorState::new(window, cx).with_placeholder("Empty note"));
        editor.update(cx, |editor, _| {
            editor.set_block_image_provider(provider);
            // An always-firing, empty auto-replace rule: without one, the
            // editor splits a `$$` that shares a line with words onto its
            // own lines while typing (`normalize_math_at`). Granit does not
            // render math, so that would only rewrite the note.
            editor.set_auto_replace(|_| Some((0..0, String::new())));
        });
        let restored = editor.update(cx, |editor, cx| load_text(editor, &body, cx));
        self.undo_floor = restored.then_some(body);
        self.editor_subscriptions = vec![
            cx.subscribe_in(&editor, window, Self::on_editor_event),
            cx.on_blur(&editor.read(cx).focus_handle(cx), window, |this, _, cx| {
                this.flush(cx);
            }),
        ];
        if had_focus {
            editor.update(cx, |editor, cx| editor.focus(window, cx));
        }
        match self.pending_restore.take() {
            Some((caret, offset)) => {
                editor.update(cx, |editor, cx| editor.set_cursor(caret, cx));
                self.scroll.set_offset(offset);
            }
            None => self.scroll.set_offset(gpui::point(px(0.), px(0.))),
        }
        self.editor = Some(editor);
        self.apply_theme(cx);
        self.mark_broken_links(cx);
        self.request_images(cx);
        self.apply_reveal(cx);
        if let Some(find) = &mut self.find {
            find.refresh(self.editor.as_ref(), cx);
        }
        cx.notify();
    }

    /// Place the caret at the pending reveal and scroll it to the top.
    fn apply_reveal(&mut self, cx: &mut Context<Self>) {
        let Some(reveal) = self.pending_reveal.take() else {
            return;
        };
        let Some(editor) = &self.editor else {
            return;
        };
        let Some(offset) = reveal.offset_in(editor.read(cx).text()) else {
            return;
        };
        editor.update(cx, |editor, cx| editor.set_cursor(offset, cx));
        self.pending_scroll = Some((offset, REVEAL_ATTEMPTS));
        cx.notify();
    }

    /// Scroll a pending offset to the top once the editor has painted it.
    /// Runs while rendering, from the last paint's layout.
    fn apply_pending_scroll(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((offset, attempts)) = self.pending_scroll else {
            return;
        };
        let top = self
            .editor
            .as_ref()
            .and_then(|editor| editor.read(cx).offset_screen_top(offset));
        match top {
            Some(top) => {
                style::scroll_row_to_top(&self.scroll, top, REVEAL_MARGIN);
                self.pending_scroll = None;
            }
            None if attempts > 0 => {
                self.pending_scroll = Some((offset, attempts - 1));
                // Render again after this frame's paint has laid it out.
                cx.on_next_frame(window, |_, _, cx| cx.notify());
            }
            None => self.pending_scroll = None,
        }
    }

    /// Restyle the editor from the active theme: live-preview colors and
    /// fonts plus code-block token colors.
    pub fn apply_theme(&mut self, cx: &mut Context<Self>) {
        let theme = cx.theme();
        let style = style::markdown_style(theme);
        self.highlighter.set_theme(theme.highlight_theme.clone());
        let hook = self.highlighter.hook();
        if let Some(editor) = &self.editor {
            editor.update(cx, |editor, cx| {
                editor.set_code_highlighter(hook);
                editor.set_markdown_style(style, cx);
            });
        }
        cx.notify();
    }

    /// Rebuild the link targets from the cave and re-mark broken links.
    fn refresh_links(&mut self, cx: &mut Context<Self>) {
        self.links = Host::core(cx)
            .with_cave(|cave| Ok(LinkIndex::from_cave(cave)))
            .unwrap_or_default();
        self.mark_broken_links(cx);
    }

    /// Start decoding the images the open note shows that are not cached.
    fn request_images(&mut self, cx: &mut Context<Self>) {
        let (Note::Loaded(loaded), Some(editor)) = (&self.note, &self.editor) else {
            return;
        };
        let text = editor.read(cx).text();
        let paths = self
            .image_cache
            .request(&loaded.images, &images::image_sources(text));
        for path in paths {
            let cave = loaded.cave.clone();
            let svg = cx.svg_renderer();
            let decode_path = path.clone();
            let task =
                cx.background_spawn(async move { images::load_image(&cave, &decode_path, svg) });
            cx.spawn(async move |this, cx| {
                let result = task.await;
                this.update(cx, |this, cx| {
                    this.image_cache.finish(path, result);
                    // Repaint with the image.
                    if let Some(editor) = &this.editor {
                        editor.update(cx, |_, cx| cx.notify());
                    }
                })
                .ok();
            })
            .detach();
        }
    }

    fn on_editor_event(
        &mut self,
        emitter: &Entity<EditorState>,
        event: &EditorEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Only the open note's editor speaks for the view.
        if self.editor.as_ref() != Some(emitter) {
            return;
        }
        match event {
            EditorEvent::Changed => {
                self.note_edited(window, cx);
                self.mark_broken_links(cx);
                self.request_images(cx);
                self.update_completion(true, window, cx);
                if let Some(find) = &mut self.find {
                    find.refresh(self.editor.as_ref(), cx);
                }
            }
            EditorEvent::SelectionChanged => {
                self.update_completion(false, window, cx);
                self.follow_caret(window, cx);
            }
            EditorEvent::OpenWikiLink(target) => self.follow_wiki_link(target, window, cx),
            EditorEvent::OpenLink(url) => links::open_external_link(url, cx),
            _ => {}
        }
    }

    /// Record a user edit of the open note: start the debounced save and
    /// keep the caret in view.
    fn note_edited(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Note::Loaded(loaded) = &mut self.note else {
            return;
        };
        let generation = loaded.session.mark_edited();
        self.autosave = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(AUTOSAVE_DELAY).await;
            this.update(cx, |this, cx| {
                let latest = matches!(
                    &this.note,
                    Note::Loaded(loaded) if loaded.session.is_latest_edit(generation)
                );
                if latest {
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
            // A pending reveal places the viewport itself.
            if this.pending_scroll.is_some() {
                return;
            }
            let Some(caret) = this
                .editor
                .as_ref()
                .and_then(|editor| editor.read(cx).caret_screen_bounds())
            else {
                return;
            };
            if style::scroll_caret_into_view(&this.scroll, caret.top(), caret.bottom()) {
                cx.notify();
            }
        });
    }

    fn on_save(&mut self, _: &SaveNote, _: &mut Window, cx: &mut Context<Self>) {
        self.flush(cx);
    }

    /// Run [`Self::after_paint`] once the editor has painted the current
    /// state. Next-frame callbacks run before that frame's paint, so this
    /// waits one frame more.
    fn schedule_after_paint(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.on_next_frame(window, |_, window, cx| {
            cx.on_next_frame(window, |this, _, cx| this.after_paint(cx));
        });
    }

    /// Re-render once the editor's layout reflects the last change, so the
    /// popup follows the caret box.
    fn after_paint(&mut self, cx: &mut Context<Self>) {
        cx.notify();
    }

    /// Title bar above the editor: the note's slug and its save state.
    fn render_header(&self, session: &NoteSession, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(SharedString::from(session.slug().to_string())),
            )
            .child(div().flex_none().text_sm().text_color(color).child(label))
    }

    fn render_editor_area(&self, editor: Entity<EditorState>, cx: &mut Context<Self>) -> gpui::Div {
        let font = Host::core(cx).lock_config().reading_font.clone();
        let theme = cx.theme();
        let (font_family, font_size) = style::editor_font(&font, theme);
        #[cfg(test)]
        self.drawn_font.set(Some((font_family.clone(), font_size)));
        let area = div()
            .min_h_full()
            .px_8()
            .py_6()
            // The editor takes its font and text color from the ambient
            // text style.
            .font_family(font_family)
            .text_size(font_size)
            .text_color(theme.foreground)
            // Runs before the editor's own press handling, so a link click
            // can be traced back to the text under the pointer.
            .capture_any_mouse_down(cx.listener(|this, event: &MouseDownEvent, _, _| {
                this.last_press = Some(event.position);
            }))
            // A press below the text focuses the editor.
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    if let Some(editor) = &this.editor {
                        let handle = editor.read(cx).focus_handle(cx);
                        if !handle.is_focused(window) {
                            window.focus(&handle, cx);
                        }
                    }
                }),
            )
            // Undo stops at the loaded body (see `undo_floor`).
            .capture_action(cx.listener(|this, _: &zorite_editor::Undo, _, cx| {
                if this.at_undo_floor(cx) {
                    cx.stop_propagation();
                }
            }))
            .child(div().max_w(px(820.)).child(editor));
        self.intercept_completion_keys(area, cx)
    }

    /// Whether an undo now would step back past the loaded body into the
    /// text the editor normalized on load.
    fn at_undo_floor(&self, cx: &App) -> bool {
        match (&self.undo_floor, &self.editor) {
            (Some(floor), Some(editor)) => editor.read(cx).text() == floor,
            _ => false,
        }
    }
}

/// Read `slug` for the editor.
fn load_note(slug: &str, cx: &App) -> Result<Loaded, granit_core::cave::CaveError> {
    Host::core(cx).with_cave(|cave| {
        let doc = cave.read_note(slug)?;
        Ok(Loaded {
            session: NoteSession::new(slug, doc.content),
            cave: cave.path().to_path_buf(),
            images: ImageBase {
                cave: cave.path().to_path_buf(),
                note_dir: cave.note_dir(slug)?,
            },
        })
    })
}

/// Write `snapshot` to the cave it was taken in.
fn write_snapshot(cave: &PathBuf, snapshot: &SaveSnapshot, cx: &mut App) -> Result<(), String> {
    if Host::cave_path(cx).as_ref() != Some(cave) {
        return Err("the note's cave is no longer open".into());
    }
    cave_ops::save_note(&snapshot.slug, &snapshot.text, cx)
        .map(|_| ())
        .map_err(|err| err.to_string())
}

/// Load `body` into a fresh `editor` so that `text()` reads it back byte for
/// byte.
///
/// `set_text` canonicalizes the text it loads (`normalize_loaded` in
/// zorite-editor): a `$$` that shares a line with words is split onto its
/// own line. That rewrite would be saved with the first edit anywhere in the
/// note, so the loaded body is put back with `replace_range`, which does not
/// normalize. The editor's undo history then holds the normalized text as
/// its bottom step, so the view refuses an undo from the loaded body (see
/// `NoteView::undo_floor`). Returns whether the body had to be put back.
fn load_text(editor: &mut EditorState, body: &str, cx: &mut Context<EditorState>) -> bool {
    editor.set_text(body, cx);
    if editor.text() == body {
        return false;
    }
    let len = editor.text().len();
    editor.replace_range(0..len, body, cx);
    editor.set_cursor(0, cx);
    true
}

impl Render for NoteView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.apply_pending_scroll(window, cx);
        let theme = cx.theme();
        let container = div()
            .key_context(KEY_CONTEXT)
            .on_action(cx.listener(Self::on_save))
            .on_action(cx.listener(Self::on_find))
            .on_action(cx.listener(Self::on_insert_link))
            .on_action(cx.listener(Self::on_toggle_task))
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col();
        let session = match &self.note {
            Note::Empty => {
                return container
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_color(theme.muted_foreground)
                            .child("Select a note in the explorer"),
                    )
                    .into_any_element();
            }
            Note::Failed { message, .. } => {
                return container
                    .items_center()
                    .justify_center()
                    .p_6()
                    .child(div().text_color(theme.danger).child(message.clone()))
                    .into_any_element();
            }
            Note::Loaded(loaded) => &loaded.session,
        };
        let header = self.render_header(session, cx);
        let find_bar = self.render_find_bar(cx);
        let body = match self.editor.clone() {
            Some(editor) => self.render_editor_area(editor, cx).into_any_element(),
            None => div().into_any_element(),
        };
        let completion = self.render_completion(cx);
        container
            .child(header)
            .children(find_bar)
            .child(
                div()
                    .id("note-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .child(body),
            )
            .children(completion)
            .into_any_element()
    }
}

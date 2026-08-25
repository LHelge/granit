mod codemirror;
mod frontmatter;
mod reader;
mod save_queue;
mod writer;

use crate::app::{components::icons::Icon, ipc};
use granit_types::{AppConfig, Document, DocumentMeta, RenderedDocument};
use leptos::leptos_dom::helpers::{set_timeout_with_handle, TimeoutHandle};
use leptos::prelude::*;
use reader::Reader;
use save_queue::{DocumentKind, PersistSnapshot, SaveQueue};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;
use writer::Writer;

/// Debounce holdoff between the last edit and the automatic save to disk.
const AUTOSAVE_HOLDOFF_MS: u64 = 2000;

enum PersistedMeta {
    Note(DocumentMeta),
    Template(DocumentMeta),
}

// ── Shared context: open next note in edit mode ────────────────────

/// How the editor should open when switching to a new note.
#[derive(Clone, Copy, Default, PartialEq)]
pub enum EditOpen {
    /// Open in read/preview mode (default).
    #[default]
    Preview,
    /// Open in edit mode with the title input focused and selected.
    EditFocusTitle,
    /// Open in edit mode with the content textarea focused.
    EditFocusContent,
}

/// Signal provided via Leptos context so any component (e.g. tree view)
/// can request a specific editor mode on the next note switch.
#[derive(Clone, Copy)]
pub struct OpenInEdit(pub RwSignal<EditOpen>);

// ── Shared state via context ───────────────────────────────────────

/// Shared reactive state for the editor, provided via Leptos context
/// so child components can `use_editor_ctx()` instead of prop drilling.
#[derive(Clone, Copy)]
pub(super) struct EditorCtx {
    pub app: crate::app::AppCtx,
    pub active_note: RwSignal<Option<Document>>,
    pub active_template: RwSignal<Option<Document>>,
    pub notes: RwSignal<Vec<DocumentMeta>>,
    pub templates: RwSignal<Vec<DocumentMeta>>,
    pub config: RwSignal<AppConfig>,
    pub editing: RwSignal<bool>,
    pub content: RwSignal<String>,
    pub title_input: RwSignal<String>,
    pub saving: RwSignal<bool>,
    pub error: RwSignal<Option<String>>,
    pub rendered_note: RwSignal<Option<RenderedDocument>>,
    /// When true, the Writer should focus and select the title input.
    pub focus_title: RwSignal<bool>,
    /// When true, the Writer should focus the content textarea.
    pub focus_content: RwSignal<bool>,
    /// When true, the Writer should open the find/replace panel.
    pub open_search: RwSignal<bool>,
    /// Shared signal for how the next note switch should open.
    open_in_edit: RwSignal<EditOpen>,
    /// Tracks the previously active document to detect real switches.
    prev_doc_key: RwSignal<Option<String>>,
    /// Frontmatter tags for the current note.
    pub tags: RwSignal<Vec<String>>,
    /// Frontmatter icon ID for the current note, e.g. `"Star"`.
    pub icon: RwSignal<Option<String>>,
    /// Frontmatter favorite flag for the current note.
    pub favorite: RwSignal<Option<bool>>,
    /// Monotonic counter used to ignore stale async render results.
    render_request_id: RwSignal<u64>,
    /// Latest-wins queue of pending saves, processed one at a time.
    save_queue: StoredValue<Rc<RefCell<SaveQueue>>, LocalStorage>,
    /// Whether the editor holds changes not yet persisted to disk.
    pub dirty: RwSignal<bool>,
    /// Pending debounced-autosave timer, if armed.
    autosave_timer: StoredValue<Cell<Option<TimeoutHandle>>, LocalStorage>,
}

impl EditorCtx {
    fn current_kind_untracked(self) -> Option<DocumentKind> {
        if self.active_template.get_untracked().is_some() {
            Some(DocumentKind::Template)
        } else if self.active_note.get_untracked().is_some() {
            Some(DocumentKind::Note)
        } else {
            None
        }
    }

    fn current_doc_key_untracked(self) -> Option<String> {
        if let Some(template) = self.active_template.get_untracked() {
            Some(format!("template:{}", template.meta.slug))
        } else {
            self.active_note
                .get_untracked()
                .map(|note| format!("note:{}", note.meta.slug))
        }
    }

    fn parse_doc_key(key: &str) -> Option<(DocumentKind, &str)> {
        key.strip_prefix("note:")
            .map(|slug| (DocumentKind::Note, slug))
            .or_else(|| {
                key.strip_prefix("template:")
                    .map(|slug| (DocumentKind::Template, slug))
            })
    }

    /// Persist a snapshot to disk and refresh the sidebar note list.
    async fn persist(self, snapshot: &PersistSnapshot) -> Result<PersistedMeta, String> {
        match snapshot.kind {
            DocumentKind::Note => {
                let meta = ipc::update_note(
                    &snapshot.slug,
                    &snapshot.name,
                    &snapshot.content,
                    snapshot.tags.clone(),
                    snapshot.icon.clone(),
                    snapshot.favorite,
                )
                .await?;
                if let Ok(notes) = ipc::fetch_notes().await {
                    self.notes.set(notes);
                }
                Ok(PersistedMeta::Note(meta))
            }
            DocumentKind::Template => {
                let meta = ipc::update_template(
                    &snapshot.slug,
                    &snapshot.name,
                    &snapshot.content,
                    snapshot.tags.clone(),
                    snapshot.icon.clone(),
                )
                .await?;
                if let Ok(templates) = ipc::fetch_templates().await {
                    self.templates.set(templates);
                }
                Ok(PersistedMeta::Template(meta))
            }
        }
    }

    /// Enqueue a snapshot for persistence. If no save is currently in flight,
    /// starts the drain loop. The loop processes snapshots one at a time and
    /// updates reactive state based on the result and whether the snapshot
    /// was explicit (Save button) or automatic (note switch).
    fn enqueue_persist(self, snapshot: PersistSnapshot) {
        let queue = self.save_queue.get_value();
        let should_drive = queue.borrow_mut().enqueue(snapshot);
        if !should_drive {
            return;
        }
        leptos::task::spawn_local(async move {
            loop {
                let next = queue.borrow_mut().take_next();
                let Some(snapshot) = next else { break };
                let result = self.persist(&snapshot).await;
                self.apply_persist_result(&snapshot, result);
            }
        });
    }

    /// Whether the current editor signals still match a persisted snapshot.
    /// Used to clear `dirty` after an auto-save: if the user kept typing
    /// while the save was in flight, the document stays dirty.
    fn snapshot_matches_current(self, snapshot: &PersistSnapshot) -> bool {
        if self.current_doc_key_untracked().as_deref() != Some(snapshot.doc_key().as_str()) {
            return false;
        }
        let title = self.title_input.get_untracked().trim().to_string();
        let effective_name = if title.is_empty() {
            snapshot.slug.clone()
        } else {
            title
        };
        snapshot.name == effective_name
            && snapshot.content == self.content.get_untracked()
            && snapshot.tags.as_ref() == Some(&self.tags.get_untracked())
            && snapshot.icon == self.icon.get_untracked()
            && snapshot.favorite == self.favorite.get_untracked()
    }

    fn apply_persist_result(
        self,
        snapshot: &PersistSnapshot,
        result: Result<PersistedMeta, String>,
    ) {
        match result {
            Ok(PersistedMeta::Note(meta)) => {
                if snapshot.explicit {
                    self.dirty.set(false);
                    self.prev_doc_key.set(Some(format!("note:{}", meta.slug)));
                    self.app.set_active_note_document(Document {
                        meta,
                        content: snapshot.content.clone(),
                    });
                    self.editing.set(false);
                } else if self.snapshot_matches_current(snapshot) {
                    self.dirty.set(false);
                }
            }
            Ok(PersistedMeta::Template(meta)) => {
                if snapshot.explicit {
                    self.dirty.set(false);
                    self.prev_doc_key
                        .set(Some(format!("template:{}", meta.slug)));
                    self.app.set_active_template_document(Document {
                        meta,
                        content: snapshot.content.clone(),
                    });
                    self.editing.set(false);
                } else if self.snapshot_matches_current(snapshot) {
                    self.dirty.set(false);
                }
            }
            Err(e) => {
                let msg = if snapshot.explicit {
                    e
                } else {
                    format!("Autosave failed: {e}")
                };
                self.error.set(Some(msg));
            }
        }
        if snapshot.explicit {
            self.saving.set(false);
        }
    }

    fn invalidate_renders(self) {
        let next = self.render_request_id.get_untracked().wrapping_add(1);
        self.render_request_id.set(next);
    }

    /// Render a note by slug and update the rendered note only if this is
    /// still the latest request for the currently active note.
    fn request_render(self, kind: DocumentKind, slug: String) {
        let request_id = self.render_request_id.get_untracked().wrapping_add(1);
        self.render_request_id.set(request_id);
        let expected_key = match kind {
            DocumentKind::Note => format!("note:{slug}"),
            DocumentKind::Template => format!("template:{slug}"),
        };

        leptos::task::spawn_local(async move {
            let rendered = match kind {
                DocumentKind::Note => ipc::render_note(&slug).await,
                DocumentKind::Template => ipc::render_template(&slug).await,
            };
            let still_latest = self.render_request_id.get_untracked() == request_id;
            let still_active =
                self.current_doc_key_untracked().as_deref() == Some(expected_key.as_str());

            if !still_latest || !still_active {
                return;
            }

            match rendered {
                Ok(rendered) => self.rendered_note.set(Some(rendered)),
                Err(_) => self.rendered_note.set(None),
            }
        });
    }

    /// Build a snapshot of the current editor state for the given document
    /// key, reading all signals synchronously. Returns `None` if the key is
    /// not a recognised `note:` or `template:` key.
    fn snapshot_for(self, doc_key: &str, explicit: bool) -> Option<PersistSnapshot> {
        let (kind, slug) = Self::parse_doc_key(doc_key)?;
        let slug = slug.to_string();
        let title = self.title_input.get_untracked().trim().to_string();
        let name = if title.is_empty() {
            slug.clone()
        } else {
            title
        };
        Some(PersistSnapshot {
            kind,
            slug,
            name,
            content: self.content.get_untracked(),
            tags: Some(self.tags.get_untracked()),
            icon: self.icon.get_untracked(),
            favorite: self.favorite.get_untracked(),
            explicit,
        })
    }

    /// Auto-save the current edits when switching away from a note.
    ///
    /// Must be called with the editor state still reflecting the old document
    /// (i.e. before `ctx.content.set(new_content)` overwrites it).
    fn auto_save(self, doc_key: String) {
        let Some(snapshot) = self.snapshot_for(&doc_key, false) else {
            return;
        };
        self.enqueue_persist(snapshot);
    }

    /// Mark the document dirty and (re)arm the debounced autosave timer.
    /// Called on every edit; the save fires once typing pauses.
    pub fn schedule_autosave(self) {
        self.dirty.set(true);
        self.cancel_autosave_timer();
        let handle = set_timeout_with_handle(
            move || {
                // try_* variants: the timer may outlive the reactive owner
                // during app teardown.
                self.autosave_timer.try_with_value(|cell| cell.set(None));
                if self.editing.try_get_untracked() == Some(true)
                    && self.dirty.try_get_untracked() == Some(true)
                {
                    self.autosave_content();
                }
            },
            Duration::from_millis(AUTOSAVE_HOLDOFF_MS),
        );
        if let Ok(handle) = handle {
            self.autosave_timer
                .with_value(|cell| cell.set(Some(handle)));
        }
    }

    pub fn cancel_autosave_timer(self) {
        if let Some(handle) = self
            .autosave_timer
            .try_with_value(|cell| cell.take())
            .flatten()
        {
            handle.clear();
        }
    }

    /// Debounced autosave: persists content, tags, icon, and favorite, but
    /// never renames — a half-typed title must not rename the file (and
    /// rewrite inbound wiki-links) on every pause. A pending rename keeps
    /// the document dirty and is applied by the full save when the user
    /// leaves edit mode, saves explicitly, or switches notes.
    fn autosave_content(self) {
        let Some(doc_key) = self.current_doc_key_untracked() else {
            return;
        };
        let Some(mut snapshot) = self.snapshot_for(&doc_key, false) else {
            return;
        };
        snapshot.name = snapshot.slug.clone();
        self.enqueue_persist(snapshot);
    }

    /// Save the current note (user-triggered via button).
    pub fn save(self) {
        let Some(doc_key) = self.current_doc_key_untracked() else {
            return;
        };
        if self.title_input.get_untracked().trim().is_empty() {
            self.error.set(Some("Filename cannot be empty".to_string()));
            return;
        }
        let Some(snapshot) = self.snapshot_for(&doc_key, true) else {
            return;
        };

        self.saving.set(true);
        self.error.set(None);
        self.enqueue_persist(snapshot);
    }

    /// Toggle between edit and preview mode.
    ///
    /// Leaving edit mode with unsaved changes runs a full save (including a
    /// pending rename); on success the save flow switches to preview and the
    /// preview renders the saved state. On failure (e.g. empty filename) the
    /// editor stays in edit mode with the error shown.
    pub fn toggle_mode(self) {
        let was_editing = self.editing.get_untracked();
        if was_editing {
            self.cancel_autosave_timer();
            if self.dirty.get_untracked() {
                self.save();
                return;
            }
        }
        self.editing.update(|v| *v = !*v);
        // Re-render when switching back to preview (content may have been edited)
        if was_editing {
            match self.current_kind_untracked() {
                Some(DocumentKind::Note) => {
                    if let Some(note) = self.active_note.get_untracked() {
                        self.request_render(DocumentKind::Note, note.meta.slug.clone());
                    }
                }
                Some(DocumentKind::Template) => {
                    if let Some(template) = self.active_template.get_untracked() {
                        self.request_render(DocumentKind::Template, template.meta.slug.clone());
                    }
                }
                None => {}
            }
        }
    }

    /// Navigate to a wiki-link target, creating the note if it's a broken link.
    ///
    /// `fragment` is a heading anchor id (`[[note#anchor]]`); when present and the
    /// target note exists, the reader scrolls to that heading after rendering.
    pub fn navigate_wiki_link(self, slug: String, fragment: Option<String>, is_broken: bool) {
        leptos::task::spawn_local(async move {
            if is_broken {
                if let Ok(meta) = ipc::create_note(&slug, None, None).await {
                    if let Ok(all) = ipc::fetch_notes().await {
                        self.notes.set(all);
                    }
                    if let Ok(note) = ipc::read_note(&meta.slug).await {
                        self.open_in_edit.set(EditOpen::EditFocusContent);
                        self.focus_content.set(true);
                        self.app.set_active_note_document(note);
                    }
                }
            } else if let Ok(note) = ipc::read_note(&slug).await {
                self.app.set_active_note_document(note);
                if let Some(anchor) = fragment {
                    crate::app::markdown_links::scroll_to_anchor(anchor);
                }
            }
        });
    }
}

/// Retrieve the editor context from a child component.
pub(super) fn use_editor_ctx() -> EditorCtx {
    expect_context::<EditorCtx>()
}

// ── Main component ─────────────────────────────────────────────────

#[component]
pub fn Editor() -> impl IntoView {
    let app = expect_context::<crate::app::AppCtx>();
    let ctx = EditorCtx {
        app,
        active_note: app.active_note,
        active_template: app.active_template,
        notes: app.notes,
        templates: app.templates,
        config: app.config,
        editing: app.editing,
        content: RwSignal::new(String::new()),
        title_input: RwSignal::new(String::new()),
        saving: RwSignal::new(false),
        error: RwSignal::new(None),
        rendered_note: RwSignal::new(None),
        focus_title: RwSignal::new(false),
        focus_content: RwSignal::new(false),
        open_search: RwSignal::new(false),
        open_in_edit: expect_context::<OpenInEdit>().0,
        prev_doc_key: RwSignal::new(None),
        tags: RwSignal::new(Vec::new()),
        icon: RwSignal::new(None),
        favorite: RwSignal::new(None),
        render_request_id: RwSignal::new(0),
        save_queue: StoredValue::new_local(Rc::new(RefCell::new(SaveQueue::new()))),
        dirty: RwSignal::new(false),
        autosave_timer: StoredValue::new_local(Cell::new(None)),
    };
    provide_context(ctx);

    // Detect real note switches: auto-save previous, render new note.
    //
    // Concurrency note: auto-save snapshots the current editor state
    // synchronously and enqueues it on the save queue (see `SaveQueue`), so
    // rapid switches and explicit saves cannot interleave and write the wrong
    // content to the wrong slug.
    Effect::new(move || {
        let new_note = ctx.active_note.get();
        let new_template = ctx.active_template.get();
        let old_key = ctx.prev_doc_key.get_untracked();
        let was_editing = ctx.editing.get_untracked();

        let new_key = if let Some(template) = new_template.as_ref() {
            Some(format!("template:{}", template.meta.slug))
        } else {
            new_note
                .as_ref()
                .map(|note| format!("note:{}", note.meta.slug))
        };
        let is_switch = old_key != new_key;

        if is_switch {
            // Auto-save the previous note when switching away in edit mode.
            // The debounce timer is cancelled first: `auto_save` snapshots
            // synchronously, and the new document must not inherit the timer.
            ctx.cancel_autosave_timer();
            ctx.dirty.set(false);
            if was_editing {
                if let Some(doc_key) = old_key {
                    ctx.auto_save(doc_key);
                }
            }
            // Open new note in preview or edit mode depending on flag
            let mode = ctx.open_in_edit.get_untracked();
            ctx.open_in_edit.set(EditOpen::Preview);
            let editing = mode != EditOpen::Preview;
            ctx.editing.set(editing);
            match mode {
                EditOpen::EditFocusTitle => ctx.focus_title.set(true),
                EditOpen::EditFocusContent => ctx.focus_content.set(true),
                EditOpen::Preview => {}
            }
            ctx.app.selected_note_text.set(None);
        }

        // Re-render whenever the note changes (switch or same-slug update)
        if let Some(template) = &new_template {
            ctx.request_render(DocumentKind::Template, template.meta.slug.clone());
        } else if let Some(note) = &new_note {
            ctx.request_render(DocumentKind::Note, note.meta.slug.clone());
        } else {
            ctx.invalidate_renders();
            ctx.rendered_note.set(None);
        }

        // Sync local editor state with the new active document
        if let Some(template) = new_template {
            ctx.prev_doc_key
                .set(Some(format!("template:{}", template.meta.slug.clone())));
            ctx.content.set(template.content.clone());
            ctx.title_input.set(template.meta.slug.clone());
            ctx.favorite.set(None);
        } else if let Some(note) = new_note {
            ctx.prev_doc_key
                .set(Some(format!("note:{}", note.meta.slug.clone())));
            ctx.content.set(note.content.clone());
            ctx.title_input.set(note.meta.slug.clone());
            ctx.favorite.set(note.meta.favorite);
        } else {
            ctx.prev_doc_key.set(None);
            ctx.content.set(String::new());
            ctx.title_input.set(String::new());
            ctx.tags.set(Vec::new());
            ctx.icon.set(None);
            ctx.favorite.set(None);
            ctx.app.selected_note_text.set(None);
        }
        ctx.error.set(None);
    });

    // Sync tags, icon, and favorite from rendered note frontmatter whenever it changes.
    Effect::new(move || {
        let fm = ctx.rendered_note.get().and_then(|r| r.frontmatter);
        let tags = fm.as_ref().map(|f| f.tags.clone()).unwrap_or_default();
        let icon = fm.as_ref().and_then(|f| f.icon.clone());
        let favorite = if ctx.active_note.get().is_some() {
            fm.and_then(|f| f.favorite)
        } else {
            None
        };
        ctx.tags.set(tags);
        ctx.icon.set(icon);
        ctx.favorite.set(favorite);
    });

    let has_document =
        move || ctx.active_note.get().is_some() || ctx.active_template.get().is_some();

    // Copy-button feedback: shows a checkmark briefly after a copy.
    let copied = RwSignal::new(false);
    let on_copy = move |_| {
        let content = ctx.content.get_untracked();
        leptos::task::spawn_local(async move {
            match ipc::copy_note_html(&content).await {
                Ok(()) => {
                    copied.set(true);
                    let _ = set_timeout_with_handle(
                        move || {
                            copied.try_set(false);
                        },
                        Duration::from_millis(1500),
                    );
                }
                Err(e) => {
                    ctx.app
                        .push_error("editor", format!("Failed to copy note: {e}"));
                }
            }
        });
    };

    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        // Note: Escape deliberately has no binding here — CodeMirror uses it
        // to close the completion popup, and an outer exit-edit-mode binding
        // would also fire on that same keypress.

        // Cmd on macOS, Ctrl on Linux/Windows.
        let is_mac = expect_context::<crate::app::AppCtx>().is_mac;
        let modifier = if is_mac { ev.meta_key() } else { ev.ctrl_key() };
        if !modifier {
            return;
        }
        match ev.key().as_str() {
            // Cmd/Ctrl+E → enter edit mode
            "e" if has_document() && !ctx.editing.get_untracked() => {
                ev.prevent_default();
                ctx.editing.set(true);
                ctx.focus_content.set(true);
            }
            // Cmd/Ctrl+S → save and return to preview
            "s" if ctx.editing.get_untracked() => {
                ev.prevent_default();
                ctx.save();
            }
            _ => {}
        }
    };

    view! {
        <main
            class="flex-1 flex flex-col overflow-hidden bg-base-100 relative outline-none"
            tabindex="-1"
            on:keydown=on_keydown
        >
            // Floating action buttons — always top-right, no layout impact
            <Show when=has_document>
                <div class="absolute top-3 right-4 z-10 flex items-center gap-1">
                    // Find/replace (edit mode): opens the editor search panel
                    <Show when=move || ctx.editing.get()>
                        <div class="tooltip tooltip-bottom" data-tip="Find in note">
                            <button
                                class="btn btn-ghost btn-xs btn-square"
                                on:click=move |_| ctx.open_search.set(true)
                            >
                                <Icon icon=icondata_lu::LuSearch width="1rem" height="1rem"/>
                            </button>
                        </div>
                    </Show>
                    // Copy rendered note (both modes): rich text for Word/
                    // Teams-style targets, markdown as plain-text fallback
                    <div
                        class="tooltip tooltip-bottom"
                        data-tip=move || if copied.get() { "Copied!" } else { "Copy note" }
                    >
                        <button class="btn btn-ghost btn-xs btn-square" on:click=on_copy>
                            <Show
                                when=move || copied.get()
                                fallback=|| view! {
                                    <Icon icon=icondata_lu::LuCopy width="1rem" height="1rem"/>
                                }
                            >
                                <span class="text-success inline-flex">
                                    <Icon icon=icondata_lu::LuCheck width="1rem" height="1rem"/>
                                </span>
                            </Show>
                        </button>
                    </div>
                    <Show
                        when=move || ctx.editing.get()
                        fallback=move || view! {
                            // Preview mode: pencil icon → switch to edit
                            <div class="tooltip tooltip-bottom" data-tip="Edit">
                                <button
                                    class="btn btn-ghost btn-xs btn-square"
                                    on:click=move |_| ctx.toggle_mode()
                                >
                                    <Icon icon=icondata_lu::LuPencil width="1rem" height="1rem"/>
                                </button>
                            </div>
                        }
                    >
                        // Edit mode: floppy disk → save, X → cancel
                        <div class="tooltip tooltip-bottom" data-tip="Save">
                            <button
                                class="btn btn-ghost btn-xs btn-square"
                                on:click=move |_| ctx.save()
                                disabled=move || ctx.saving.get()
                            >
                                <Icon icon=icondata_lu::LuSave width="1rem" height="1rem"/>
                            </button>
                        </div>
                        <div class="tooltip tooltip-bottom" data-tip="Close editor">
                            <button
                                class="btn btn-ghost btn-xs btn-square"
                                on:click=move |_| ctx.toggle_mode()
                            >
                                <Icon icon=icondata_lu::LuX width="1rem" height="1rem"/>
                            </button>
                        </div>
                    </Show>
                </div>
            </Show>

            // Error banner
            <Show when=move || ctx.error.get().is_some()>
                <div class="px-4 py-1.5 bg-error/15 border-b border-error/30 text-error/80 text-xs flex items-center gap-2 shrink-0">
                    <span class="flex-1">{move || ctx.error.get().unwrap_or_default()}</span>
                    <button
                        class="btn btn-ghost btn-xs btn-square text-error/70"
                        on:click=move |_| ctx.error.set(None)
                    >
                        "✕"
                    </button>
                </div>
            </Show>

            // Content area — same padding and layout for both modes
            <div class="flex-1 overflow-y-auto px-8 pt-8 flex flex-col min-h-0">
                <Show
                    when=has_document
                    fallback=|| view! {
                        <p class="text-base-content/35 italic">"Select or create a note or template to get started"</p>
                    }
                >
                    <div class="prose max-w-none flex-1 flex flex-col min-h-0">
                        <Show
                            when=move || ctx.editing.get()
                            fallback=move || view! { <Reader /> }
                        >
                            <Writer />
                        </Show>
                    </div>
                </Show>
            </div>
        </main>
    }
}

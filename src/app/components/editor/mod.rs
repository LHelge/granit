mod frontmatter;
mod reader;
pub(crate) mod text_editing;
mod writer;

use leptos::prelude::*;

use granit_types::{AppConfig, Note, NoteMeta, RenderedNote};

use reader::Reader;
use writer::Writer;

use super::icons::{PencilIcon, SaveIcon, XCloseIcon};
use crate::app::ipc;

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
    pub active_note: RwSignal<Option<Note>>,
    pub notes: RwSignal<Vec<NoteMeta>>,
    pub config: RwSignal<AppConfig>,
    pub editing: RwSignal<bool>,
    pub content: RwSignal<String>,
    pub title_input: RwSignal<String>,
    pub saving: RwSignal<bool>,
    pub error: RwSignal<Option<String>>,
    pub rendered_note: RwSignal<Option<RenderedNote>>,
    /// When true, the Writer should focus and select the title input.
    pub focus_title: RwSignal<bool>,
    /// When true, the Writer should focus the content textarea.
    pub focus_content: RwSignal<bool>,
    /// Shared signal for how the next note switch should open.
    open_in_edit: RwSignal<EditOpen>,
    /// Tracks the slug of the previously active note to detect real switches.
    prev_slug: RwSignal<Option<String>>,
    /// Frontmatter tags for the current note.
    pub tags: RwSignal<Vec<String>>,
}

impl EditorCtx {
    /// Persist a note to disk and refresh the sidebar note list.
    async fn persist(
        self,
        slug: &str,
        name: &str,
        content: &str,
        tags: Option<Vec<String>>,
    ) -> Result<NoteMeta, String> {
        let meta = ipc::update_note(slug, name, content, tags).await?;
        if let Ok(notes) = ipc::fetch_notes().await {
            self.notes.set(notes);
        }
        Ok(meta)
    }

    /// Render a note by slug and update the rendered_note signal.
    async fn render(self, slug: &str) {
        match ipc::render_note(slug).await {
            Ok(rendered) => self.rendered_note.set(Some(rendered)),
            Err(_) => self.rendered_note.set(None),
        }
    }

    /// Auto-save the current edits when switching away from a note.
    fn auto_save(self, slug: String) {
        let content = self.content.get_untracked();
        let title = self.title_input.get_untracked().trim().to_string();
        let tags = self.tags.get_untracked();
        let name = if title.is_empty() {
            slug.clone()
        } else {
            title
        };
        leptos::task::spawn_local(async move {
            if let Err(e) = self.persist(&slug, &name, &content, Some(tags)).await {
                self.error.set(Some(format!("Autosave failed: {e}")));
            }
        });
    }

    /// Save the current note (user-triggered via button).
    pub fn save(self) {
        let Some(note) = self.active_note.get_untracked() else {
            return;
        };
        let content = self.content.get_untracked();
        let name = self.title_input.get_untracked().trim().to_string();
        if name.is_empty() {
            self.error.set(Some("Filename cannot be empty".to_string()));
            return;
        }

        self.saving.set(true);
        self.error.set(None);
        let old_slug = note.meta.slug.clone();
        let tags = self.tags.get_untracked();

        leptos::task::spawn_local(async move {
            match self.persist(&old_slug, &name, &content, Some(tags)).await {
                Ok(meta) => {
                    self.prev_slug.set(Some(meta.slug.clone()));
                    let slug = meta.slug.clone();
                    self.active_note.set(Some(Note { meta, content }));
                    self.editing.set(false);
                    self.render(&slug).await;
                }
                Err(e) => self.error.set(Some(e)),
            }
            self.saving.set(false);
        });
    }

    /// Toggle between edit and preview mode.
    pub fn toggle_mode(self) {
        let was_editing = self.editing.get_untracked();
        self.editing.update(|v| *v = !*v);
        // Re-render when switching back to preview (content may have been edited)
        if was_editing {
            if let Some(note) = self.active_note.get_untracked() {
                let slug = note.meta.slug.clone();
                leptos::task::spawn_local(async move {
                    self.render(&slug).await;
                });
            }
        }
    }

    /// Navigate to a wiki-link target, creating the note if it's a broken link.
    pub fn navigate_wiki_link(self, slug: String, is_broken: bool) {
        leptos::task::spawn_local(async move {
            if is_broken {
                if let Ok(meta) = ipc::create_note(&slug, None).await {
                    if let Ok(all) = ipc::fetch_notes().await {
                        self.notes.set(all);
                    }
                    if let Ok(note) = ipc::read_note(&meta.slug).await {
                        self.open_in_edit.set(EditOpen::EditFocusContent);
                        self.focus_content.set(true);
                        self.active_note.set(Some(note));
                    }
                }
            } else if let Ok(note) = ipc::read_note(&slug).await {
                self.active_note.set(Some(note));
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
        active_note: app.active_note,
        notes: app.notes,
        config: app.config,
        editing: RwSignal::new(false),
        content: RwSignal::new(String::new()),
        title_input: RwSignal::new(String::new()),
        saving: RwSignal::new(false),
        error: RwSignal::new(None),
        rendered_note: RwSignal::new(None),
        focus_title: RwSignal::new(false),
        focus_content: RwSignal::new(false),
        open_in_edit: expect_context::<OpenInEdit>().0,
        prev_slug: RwSignal::new(None),
        tags: RwSignal::new(Vec::new()),
    };
    provide_context(ctx);

    // Detect real note switches: auto-save previous, render new note.
    Effect::new(move || {
        let new_note = ctx.active_note.get();
        let old_slug = ctx.prev_slug.get_untracked();
        let was_editing = ctx.editing.get_untracked();
        let is_saving = ctx.saving.get_untracked();

        let new_slug = new_note.as_ref().map(|n| n.meta.slug.clone());
        let is_switch = old_slug != new_slug && !is_saving;

        if is_switch {
            // Auto-save the previous note when switching away in edit mode
            if was_editing {
                if let Some(slug) = old_slug {
                    ctx.auto_save(slug);
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

            match &new_note {
                Some(note) => {
                    let slug = note.meta.slug.clone();
                    leptos::task::spawn_local(async move {
                        ctx.render(&slug).await;
                    });
                }
                None => ctx.rendered_note.set(None),
            }
        }

        // Sync local editor state with the new active note
        if let Some(note) = new_note {
            ctx.prev_slug.set(Some(note.meta.slug.clone()));
            ctx.content.set(note.content.clone());
            ctx.title_input.set(note.meta.slug.clone());
        } else {
            ctx.prev_slug.set(None);
            ctx.content.set(String::new());
            ctx.title_input.set(String::new());
            ctx.tags.set(Vec::new());
        }
        ctx.error.set(None);
    });

    // Sync tags from rendered note frontmatter whenever it changes.
    Effect::new(move || {
        let tags = ctx
            .rendered_note
            .get()
            .and_then(|r| r.frontmatter)
            .map(|fm| fm.tags)
            .unwrap_or_default();
        ctx.tags.set(tags);
    });

    let has_note = move || ctx.active_note.get().is_some();

    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        // Escape → cancel editing (no modifier needed)
        if ev.key() == "Escape" && ctx.editing.get_untracked() {
            ev.prevent_default();
            ctx.toggle_mode();
            return;
        }

        // Cmd on macOS, Ctrl on Linux/Windows.
        let is_mac = expect_context::<crate::app::AppCtx>().is_mac;
        let modifier = if is_mac { ev.meta_key() } else { ev.ctrl_key() };
        if !modifier {
            return;
        }
        match ev.key().as_str() {
            // Cmd/Ctrl+E → enter edit mode
            "e" => {
                if ctx.active_note.get_untracked().is_some() && !ctx.editing.get_untracked() {
                    ev.prevent_default();
                    ctx.editing.set(true);
                    ctx.focus_content.set(true);
                }
            }
            // Cmd/Ctrl+S → save and return to preview
            "s" => {
                if ctx.editing.get_untracked() {
                    ev.prevent_default();
                    ctx.save();
                }
            }
            _ => {}
        }
    };

    view! {
        <main
            class="flex-1 flex flex-col overflow-hidden bg-stone-900 relative outline-none"
            tabindex="-1"
            on:keydown=on_keydown
        >
            // Floating action buttons — always top-right, no layout impact
            <Show when=has_note>
                <div class="absolute top-3 right-4 z-10 flex items-center gap-1">
                    <Show
                        when=move || ctx.editing.get()
                        fallback=move || view! {
                            // Preview mode: pencil icon → switch to edit
                            <button
                                class="p-1.5 rounded text-stone-500 hover:text-stone-200 hover:bg-stone-700 transition-colors"
                                title="Edit"
                                on:click=move |_| ctx.toggle_mode()
                            >
                                <PencilIcon />
                            </button>
                        }
                    >
                        // Edit mode: floppy disk → save, X → cancel
                        <button
                            class="p-1.5 rounded text-stone-500 hover:text-stone-200 hover:bg-stone-700 transition-colors disabled:opacity-30"
                            title="Save"
                            on:click=move |_| ctx.save()
                            disabled=move || ctx.saving.get()
                        >
                            <SaveIcon />
                        </button>
                        <button
                            class="p-1.5 rounded text-stone-500 hover:text-stone-200 hover:bg-stone-700 transition-colors"
                            title="Cancel editing"
                            on:click=move |_| ctx.toggle_mode()
                        >
                            <XCloseIcon />
                        </button>
                    </Show>
                </div>
            </Show>

            // Error banner
            <Show when=move || ctx.error.get().is_some()>
                <div class="px-4 py-1.5 bg-red-900/50 border-b border-red-700 text-red-300 text-xs flex items-center gap-2 shrink-0">
                    <span class="flex-1">{move || ctx.error.get().unwrap_or_default()}</span>
                    <button
                        class="text-red-400 hover:text-red-200"
                        on:click=move |_| ctx.error.set(None)
                    >
                        "✕"
                    </button>
                </div>
            </Show>

            // Content area — same padding and layout for both modes
            <div class="flex-1 overflow-y-auto px-8 pt-8 flex flex-col min-h-0">
                <Show
                    when=has_note
                    fallback=|| view! {
                        <p class="text-stone-500 italic">"Select or create a note to get started"</p>
                    }
                >
                    <div class="prose prose-invert max-w-none flex-1 flex flex-col min-h-0">
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

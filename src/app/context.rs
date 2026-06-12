use crate::app::ipc;
use leptos::prelude::*;

// ── App-wide shared state via context ──────────────────────────────

/// A single error entry in the unified error channel.
#[derive(Clone, PartialEq)]
pub struct AppError {
    pub id: u32,
    pub source: &'static str,
    pub message: String,
}

/// Shared reactive state provided via Leptos context so child components can
/// `expect_context::<AppCtx>()` instead of receiving these signals as props.
#[derive(Clone, Copy)]
pub struct AppCtx {
    pub config: RwSignal<granit_types::AppConfig>,
    pub notes: RwSignal<Vec<granit_types::DocumentMeta>>,
    pub templates: RwSignal<Vec<granit_types::DocumentMeta>>,
    pub folders: RwSignal<Vec<String>>,
    pub active_note: RwSignal<Option<granit_types::Document>>,
    pub active_template: RwSignal<Option<granit_types::Document>>,
    pub selected_note_text: RwSignal<Option<String>>,
    /// Whether the editor is currently in edit (writing) mode. Owned by the
    /// editor but lifted here so the app-root `cave:notes-changed` listener can
    /// avoid reconciling the active note while the editor is mid-edit/save.
    pub editing: RwSignal<bool>,
    pub is_mac: bool,
    errors: RwSignal<Vec<AppError>>,
    next_id: RwSignal<u32>,
}

impl AppCtx {
    /// Create a new `AppCtx` with default values.
    pub fn new(is_mac: bool) -> Self {
        Self {
            config: RwSignal::new(granit_types::AppConfig::default()),
            notes: RwSignal::new(Vec::new()),
            templates: RwSignal::new(Vec::new()),
            folders: RwSignal::new(Vec::new()),
            active_note: RwSignal::new(None),
            active_template: RwSignal::new(None),
            selected_note_text: RwSignal::new(None),
            editing: RwSignal::new(false),
            is_mac,
            errors: RwSignal::new(Vec::new()),
            next_id: RwSignal::new(0),
        }
    }

    /// Push an error and return its id (for later dismissal).
    pub fn push_error(&self, source: &'static str, message: impl Into<String>) -> u32 {
        let id = self.next_id.get_untracked();
        self.next_id.set(id + 1);
        self.errors.update(|v| {
            v.push(AppError {
                id,
                source,
                message: message.into(),
            })
        });
        // Auto-dismiss after 8 seconds
        let ctx = *self;
        leptos::task::spawn_local(async move {
            gloo_timers::future::sleep(std::time::Duration::from_secs(8)).await;
            ctx.dismiss(id);
        });
        id
    }

    /// Dismiss a single error by id.
    pub fn dismiss(&self, id: u32) {
        self.errors.update(|v| v.retain(|e| e.id != id));
    }

    /// Remove all errors from a given source.
    pub fn clear_source(&self, source: &'static str) {
        self.errors.update(|v| v.retain(|e| e.source != source));
    }

    /// Get the first error for a source (for inline display).
    pub fn first_error_for(&self, source: &'static str) -> Option<String> {
        self.errors
            .get()
            .iter()
            .find(|e| e.source == source)
            .map(|e| e.message.clone())
    }

    /// Read the errors signal (for rendering the toast list).
    pub fn errors(&self) -> RwSignal<Vec<AppError>> {
        self.errors
    }

    /// Set the active DaisyUI theme by writing `data-theme` on the `<html>` element.
    pub fn set_theme(&self, name: &str) {
        let Some(root) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.document_element())
        else {
            return;
        };
        let _ = root.set_attribute("data-theme", name);
    }

    pub fn set_active_note_document(&self, note: granit_types::Document) {
        self.active_template.set(None);
        self.active_note.set(Some(note));
    }

    pub fn set_active_template_document(&self, template: granit_types::Document) {
        self.active_note.set(None);
        self.active_template.set(Some(template));
        self.selected_note_text.set(None);
    }

    pub fn clear_active_document(&self) {
        self.active_note.set(None);
        self.active_template.set(None);
        self.selected_note_text.set(None);
    }

    /// Fetch the note list into `self.notes`, surfacing failures as a toast.
    ///
    /// Replaces any previous error from the same source so repeated refreshes
    /// don't stack stale toasts.
    pub async fn refresh_notes(self) {
        self.clear_source("notes");
        match ipc::fetch_notes().await {
            Ok(notes) => self.notes.set(notes),
            Err(e) => {
                self.push_error("notes", format!("Failed to load notes: {e}"));
            }
        }
    }

    /// Fetch the folder list into `self.folders`, surfacing failures as a toast.
    pub async fn refresh_folders(self) {
        self.clear_source("folders");
        match ipc::fetch_folders().await {
            Ok(folders) => self.folders.set(folders),
            Err(e) => {
                self.push_error("folders", format!("Failed to load folders: {e}"));
            }
        }
    }

    /// Fetch the template list into `self.templates`, surfacing failures as a toast.
    pub async fn refresh_templates(self) {
        self.clear_source("templates");
        match ipc::fetch_templates().await {
            Ok(templates) => self.templates.set(templates),
            Err(e) => {
                self.push_error("templates", format!("Failed to load templates: {e}"));
            }
        }
    }

    /// Open a cave through IPC and refresh all frontend state that depends on it.
    pub async fn open_cave_and_refresh(self, path: &str) -> Result<(), String> {
        let new_config = ipc::open_cave(path).await?;
        self.config.set(new_config);

        self.refresh_notes().await;
        self.refresh_folders().await;
        self.refresh_templates().await;

        self.clear_active_document();
        Ok(())
    }
}

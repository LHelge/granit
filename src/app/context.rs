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
    pub notes: RwSignal<Vec<granit_types::NoteMeta>>,
    pub folders: RwSignal<Vec<String>>,
    pub active_note: RwSignal<Option<granit_types::Note>>,
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
            folders: RwSignal::new(Vec::new()),
            active_note: RwSignal::new(None),
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
}

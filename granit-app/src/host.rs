//! The app-wide host state: the core, and what the host adds around it.
//!
//! [`Host`] is a GPUI global set once at startup. Views reach the core with
//! [`Host::core`] (`Arc<CoreState>`, safe to move into tokio or background
//! tasks) and change the cave through [`crate::cave_ops`], which also keeps
//! the RAG index current and announces the change on
//! [`crate::events::CaveEvents`].

use std::path::PathBuf;
use std::sync::Arc;

use gpui::{App, Global};
use granit_core::open::EmbeddingModelDirs;
use granit_core::CoreState;
use granit_types::AppConfig;

use crate::config_ops::ConfigEvents;
use crate::events::CaveEvents;
use crate::last_cave::LastCaveStore;

/// Bundle identifier, shared with the Tauri app: config and data
/// directories are named after it.
pub const APP_ID: &str = "se.lhelge.granit";

/// The user-facing product name.
pub const PRODUCT_NAME: &str = "Granit";

/// How the host is set up; [`HostOptions::from_environment`] for the app,
/// [`HostOptions::default`] (no persistence, no RAG index) for tests.
#[derive(Clone, Debug, Default)]
pub struct HostOptions {
    /// Where the last-open cave is remembered. `None` neither reads nor
    /// writes it: a cave given on the command line must never replace the
    /// remembered one.
    pub last_cave: Option<LastCaveStore>,
    /// Where the RAG embedding model lives. `None` skips building the
    /// vector index.
    pub embedding_models: Option<EmbeddingModelDirs>,
}

impl HostOptions {
    /// The options of a normal launch. `remember_cave` is false when the cave
    /// comes from the command line.
    pub fn from_environment(remember_cave: bool) -> Self {
        Self {
            last_cave: remember_cave.then(LastCaveStore::in_config_dir).flatten(),
            embedding_models: default_embedding_models(),
        }
    }
}

/// Embedding models are cached in `<data dir>/se.lhelge.granit/models`, the
/// directory the Tauri app uses, so a model is downloaded once for both.
/// Nothing is bundled yet.
fn default_embedding_models() -> Option<EmbeddingModelDirs> {
    dirs::data_dir().map(|dir| EmbeddingModelDirs {
        models_dir: dir.join(APP_ID).join("models"),
        bundled_model_dir: None,
    })
}

/// The app-wide host state.
pub struct Host {
    core: Arc<CoreState>,
    options: HostOptions,
}

impl Global for Host {}

impl Host {
    /// Set up the host and the cave- and config-event emitters. Call once, after
    /// `gpui_kit::init`.
    pub fn init(options: HostOptions, cx: &mut App) {
        CaveEvents::init(cx);
        ConfigEvents::init(cx);
        cx.set_global(Host {
            core: Arc::new(CoreState::new(AppConfig::default())),
            options,
        });
    }

    fn get(cx: &App) -> &Host {
        cx.global::<Host>()
    }

    /// The core state: config, open cave, agent and vector index.
    pub fn core(cx: &App) -> Arc<CoreState> {
        Self::get(cx).core.clone()
    }

    /// Where the last-open cave is remembered, when it is.
    pub fn last_cave(cx: &App) -> Option<LastCaveStore> {
        Self::get(cx).options.last_cave.clone()
    }

    /// Where the RAG embedding model lives, when the index is built.
    pub fn embedding_models(cx: &App) -> Option<EmbeddingModelDirs> {
        Self::get(cx).options.embedding_models.clone()
    }

    /// The path of the open cave.
    pub fn cave_path(cx: &App) -> Option<PathBuf> {
        Self::get(cx).core.active_cave_path()
    }
}

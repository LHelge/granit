//! The host-agnostic open-cave flow and the RAG vector-index build.
//!
//! Opening a cave is two steps so a host can do its own work in between
//! (the Tauri app closes the presentation window and persists the path):
//! [`prepare_cave`] opens and seeds the directory without touching any
//! state, and [`CoreState::install_cave`] makes it the open cave. After
//! that, [`sync_vector_index`] starts or drops the RAG index for the new
//! cave's config.

use std::path::PathBuf;
use std::sync::Arc;

use granit_types::{AppConfig, RagConfig};
use log::warn;

use crate::agent::vectordb::{resolve_model_name, CaveVectorIndex};
use crate::cave::{Cave, CaveError};
use crate::CoreState;

/// A cave opened and seeded by [`prepare_cave`], with its loaded config,
/// ready to be installed into a [`CoreState`].
pub struct PreparedCave {
    pub cave: Cave,
    pub config: AppConfig,
}

/// Open the cave at `path` and prepare it for use: write a default
/// `.granit/config.yml` when there is none, load the config, seed
/// `.granit/agent/system.md` (migrating the legacy `agent.system_prompt`
/// config field) and seed the default presentation templates.
///
/// Touches no host state, so it is safe to run off the UI thread. Seeding is
/// best-effort; only opening the cave and reading or creating its config can
/// fail.
pub fn prepare_cave(path: PathBuf) -> Result<PreparedCave, CaveError> {
    let mut cave = Cave::open(path)?;
    cave.ensure_config()?;
    let mut config = cave.load_config()?;
    ensure_system_prompt_file(&cave, &mut config);
    ensure_presentation_defaults(&mut cave);
    Ok(PreparedCave { cave, config })
}

impl CoreState {
    /// Make `prepared` the open cave: replace the config and the cave, and
    /// reset the agent so the next message is built for the new cave. The
    /// vector index is left alone; call [`sync_vector_index`] next.
    pub fn install_cave(&self, prepared: PreparedCave) {
        *self.lock_config() = prepared.config;
        self.set_cave(Some(prepared.cave));
        self.reset_agent();
    }
}

/// Where the embedding model for the RAG index comes from: `models_dir`
/// caches downloaded models; `bundled_model_dir`, when set, holds the
/// default model shipped with the app.
#[derive(Clone, Debug)]
pub struct EmbeddingModelDirs {
    pub models_dir: PathBuf,
    pub bundled_model_dir: Option<PathBuf>,
}

/// Bring the vector index in line with the open cave's RAG config: build it
/// in the background when RAG is enabled, drop it otherwise.
pub fn sync_vector_index(
    state: &Arc<CoreState>,
    dirs: &EmbeddingModelDirs,
    runtime: &tokio::runtime::Handle,
) {
    let rag_config = state.lock_config().agent.rag.clone();
    if rag_config.enabled {
        build_vector_index(state, &rag_config, dirs, runtime);
    } else {
        state.set_vector_index(None);
    }
}

/// Build the vector index for RAG on a background task of `runtime`.
///
/// Loading the ONNX model can take seconds, so model load, cache read, and
/// the rebuild all run in the background. The current index is dropped
/// immediately (cancelling any in-flight rebuild) so an agent built during
/// the window never holds an index with stale embeddings; the new index is
/// installed once ready.
pub fn build_vector_index(
    state: &Arc<CoreState>,
    rag_config: &RagConfig,
    dirs: &EmbeddingModelDirs,
    runtime: &tokio::runtime::Handle,
) {
    let models_dir = dirs.models_dir.clone();
    let _ = std::fs::create_dir_all(&models_dir);
    let model_name = resolve_model_name(rag_config.embedding_model.as_deref()).to_string();
    let bundled_model_dir = dirs
        .bundled_model_dir
        .clone()
        .filter(|dir| dir.join("model.onnx").exists());

    state.set_vector_index(None);

    let shared_cave = state.shared_cave();
    let state = Arc::clone(state);
    runtime.spawn(async move {
        let built = tokio::task::spawn_blocking(move || {
            CaveVectorIndex::new(
                shared_cave,
                &model_name,
                &models_dir,
                bundled_model_dir.as_deref(),
            )
        })
        .await;

        let index = match built {
            Ok(Ok(index)) => index,
            Ok(Err(e)) => {
                warn!("vector index init failed (RAG disabled): {e}");
                return;
            }
            Err(e) => {
                warn!("vector index init task panicked: {e}");
                return;
            }
        };

        state.set_vector_index(Some(index.clone()));
        // ensure_agent snapshots the vector index when it builds the agent,
        // so an agent built before the index was ready would never see RAG.
        // Reset so the next message rebuilds it with the index.
        state.reset_agent();

        if let Err(e) = index.rebuild().await {
            warn!("vector index rebuild failed: {e}");
        }
    });
}

/// Ensure `.granit/agent/system.md` exists for an opening cave.
///
/// A missing file is seeded from the legacy `agent.system_prompt` config
/// field (which is then cleared from the config — the one-time migration) or,
/// absent that, from the built-in default template. An existing file is never
/// overwritten. Best-effort: on failure the agent falls back to the built-in
/// default at build time, and an unmigrated legacy value is kept in config.
fn ensure_system_prompt_file(cave: &Cave, config: &mut AppConfig) {
    if !cave.system_prompt_path().exists() {
        let content = match config.agent.system_prompt.as_deref() {
            Some(custom) if !custom.trim().is_empty() => custom,
            _ => granit_types::DEFAULT_SYSTEM_PROMPT_TEMPLATE,
        };
        if let Err(err) = cave.write_system_prompt(content) {
            warn!("Failed to seed system prompt file: {err}");
            return;
        }
    }
    if config.agent.system_prompt.is_some() {
        config.agent.system_prompt = None;
        if let Err(err) = cave.save_config(config) {
            warn!("Failed to clear migrated system prompt from config: {err}");
        }
    }
}

/// Seed the default presentation templates into a cave that has no
/// `.granit/presentations/` directory yet. Best-effort: a cave that cannot
/// be written simply has no templates until the user adds some.
fn ensure_presentation_defaults(cave: &mut Cave) {
    if let Err(err) = cave.seed_presentation_defaults() {
        warn!("Failed to seed presentation templates: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prepare(dir: &tempfile::TempDir) -> PreparedCave {
        prepare_cave(dir.path().to_path_buf()).unwrap()
    }

    #[test]
    fn prepare_cave_writes_a_default_config() {
        let dir = tempfile::tempdir().unwrap();
        let prepared = prepare(&dir);
        assert!(dir.path().join(".granit/config.yml").exists());
        assert_eq!(prepared.cave.path(), dir.path());
    }

    #[test]
    fn prepare_cave_fails_for_a_duplicate_slug() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("dup.md"), "a").unwrap();
        std::fs::write(dir.path().join("sub/dup.md"), "b").unwrap();
        assert!(matches!(
            prepare_cave(dir.path().to_path_buf()),
            Err(CaveError::DuplicateSlug { .. })
        ));
    }

    #[test]
    fn system_prompt_file_is_seeded_with_the_default() {
        let dir = tempfile::tempdir().unwrap();
        prepare(&dir);
        let seeded = std::fs::read_to_string(dir.path().join(".granit/agent/system.md")).unwrap();
        assert_eq!(seeded, granit_types::DEFAULT_SYSTEM_PROMPT_TEMPLATE);
    }

    #[test]
    fn presentation_defaults_are_seeded_only_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        prepare(&dir);
        let presentations = dir.path().join(".granit/presentations");
        assert!(presentations.join("default-dark.css").exists());
        assert!(presentations.join("default-light.css").exists());

        // A deleted default stays deleted across reopen.
        std::fs::remove_file(presentations.join("default-light.css")).unwrap();
        prepare(&dir);
        assert!(!presentations.join("default-light.css").exists());
    }

    #[test]
    fn legacy_system_prompt_config_is_migrated_to_file() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        cave.ensure_config().unwrap();
        let mut config = cave.load_config().unwrap();
        config.agent.system_prompt = Some("my custom prompt".to_string());
        cave.save_config(&config).unwrap();

        let prepared = prepare(&dir);

        // The legacy value seeds the file and is cleared from the config,
        // in memory and on disk.
        let seeded = std::fs::read_to_string(dir.path().join(".granit/agent/system.md")).unwrap();
        assert_eq!(seeded, "my custom prompt");
        assert!(prepared.config.agent.system_prompt.is_none());
        assert!(cave.load_config().unwrap().agent.system_prompt.is_none());
    }

    #[test]
    fn existing_system_prompt_file_is_never_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        cave.ensure_config().unwrap();
        cave.write_system_prompt("user-edited prompt").unwrap();
        // Even a lingering legacy config value must not clobber the file.
        let mut config = cave.load_config().unwrap();
        config.agent.system_prompt = Some("stale legacy value".to_string());
        cave.save_config(&config).unwrap();

        let prepared = prepare(&dir);

        let content = std::fs::read_to_string(dir.path().join(".granit/agent/system.md")).unwrap();
        assert_eq!(content, "user-edited prompt");
        assert!(prepared.config.agent.system_prompt.is_none());
    }

    #[test]
    fn install_cave_sets_config_and_cave_and_resets_the_agent() {
        let dir = tempfile::tempdir().unwrap();
        let mut prepared = prepare(&dir);
        prepared.config.theme = "latte".into();
        let state = CoreState::new(AppConfig::default());

        state.install_cave(prepared);

        assert_eq!(state.active_cave_path(), Some(dir.path().to_path_buf()));
        assert_eq!(state.lock_config().theme, "latte");
        assert_eq!(state.agent_generation(), 1);
    }

    #[tokio::test]
    async fn sync_vector_index_drops_the_index_when_rag_is_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let mut prepared = prepare(&dir);
        prepared.config.agent.rag.enabled = false;
        let state = Arc::new(CoreState::new(AppConfig::default()));
        state.install_cave(prepared);
        let dirs = EmbeddingModelDirs {
            models_dir: dir.path().join("models"),
            bundled_model_dir: None,
        };

        sync_vector_index(&state, &dirs, &tokio::runtime::Handle::current());

        assert!(state.vector_index().is_none());
        assert!(!dirs.models_dir.exists(), "no model is fetched");
    }
}

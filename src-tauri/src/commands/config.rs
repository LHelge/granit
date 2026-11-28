use super::store::Store;
use super::AppState;
use crate::agent::{self, AgentError};
use crate::cave::CaveError;
use granit_core::config::{rag_index_action, RagIndexAction};
use granit_core::open::{build_vector_index, prepare_cave, sync_vector_index, EmbeddingModelDirs};
use granit_types::{AppConfig, AppMetadata, ModelInfo, ProviderConfig, SidebarConfig};
use log::warn;
use std::path::PathBuf;
use tauri::Manager;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yml::Error),

    #[error("Validation error: {0}")]
    Validation(String),
}

impl serde::Serialize for ConfigError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, PartialEq)]
enum RestoreOutcome {
    NothingStored,
    InvalidPath,
    Restored,
    /// The stored path exists but the cave could not be opened (folder
    /// permissions, a duplicate slug/anchor, …). Non-fatal: the app starts
    /// with no cave open.
    Failed,
}

fn restore_cave_logic(
    path: Option<PathBuf>,
    state: &AppState,
) -> Result<RestoreOutcome, CaveError> {
    let Some(path) = path else {
        return Ok(RestoreOutcome::NothingStored);
    };

    if !path.is_dir() {
        return Ok(RestoreOutcome::InvalidPath);
    }

    state.install_cave(prepare_cave(path)?);

    Ok(RestoreOutcome::Restored)
}

/// Restore the cave at `path` into `state`, **never failing**.
///
/// A cave that cannot be opened — folder permissions (`EPERM`), a duplicate
/// slug, a duplicate heading anchor, etc. — degrades to
/// [`RestoreOutcome::Failed`] with no cave set, instead of propagating the
/// error. This is the invariant that keeps the setup hook safe: it runs inside
/// tao's non-unwinding `did_finish_launching`, so a propagated `Err` aborts the
/// whole process. The caller preserves the stored path so a later launch can
/// retry once the underlying issue is resolved.
fn restore_cave_or_empty(path: Option<PathBuf>, state: &AppState) -> RestoreOutcome {
    match restore_cave_logic(path, state) {
        Ok(outcome) => outcome,
        Err(err) => {
            warn!("Failed to restore last cave, starting with no cave: {err}");
            RestoreOutcome::Failed
        }
    }
}

pub(crate) fn restore_active_cave(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // The setup hook must never fail: a returned `Err` aborts the whole app
    // (tao calls this from `did_finish_launching`, which cannot unwind, so the
    // panic becomes a hard abort). Restoring the previously-open cave is
    // best-effort — any failure (e.g. a macOS folder-permission `EPERM` on a
    // freshly-built binary, or a since-deleted directory) degrades to starting
    // with no cave open, which the user can reopen via the dialog.
    let store = Store::new(app);
    let path = match store.load_persisted_active_cave() {
        Ok(path) => path,
        Err(err) => {
            warn!("Failed to read persisted active cave, starting with no cave: {err}");
            return Ok(());
        }
    };
    let state = app.state::<AppState>();

    // Best-effort: a `Failed` outcome keeps the stored path so a later launch
    // can retry once the underlying issue (e.g. granting folder access) is fixed.
    let outcome = restore_cave_or_empty(path, &state);

    if outcome == RestoreOutcome::InvalidPath {
        if let Err(err) = store.clear_persisted_active_cave() {
            warn!("Failed to clear stored cave path: {err}");
        }
    }

    // Build the vector index for RAG if a cave was restored.
    if outcome == RestoreOutcome::Restored {
        sync_vector_index(
            state.core(),
            &embedding_model_dirs(app),
            tauri::async_runtime::handle().inner(),
        );
    }

    Ok(())
}

fn get_config_for_state(state: &AppState) -> AppConfig {
    let config = state.lock_config();
    state.ipc_response(&config)
}

/// Lock the config, apply `f`, persist to the cave, reset the agent, and
/// return the IPC response.
///
/// `f` must validate before mutating: returning an error from it leaves the
/// stored config untouched and the agent generation unchanged.
fn update_agent_config(
    state: &AppState,
    f: impl FnOnce(&mut AppConfig) -> Result<(), ConfigError>,
) -> Result<AppConfig, ConfigError> {
    let response = {
        let mut config = state.lock_config();
        f(&mut config)?;
        config.clone()
    };
    state.save_config_to_cave(&response)?;
    state.reset_agent();
    Ok(state.ipc_response(&response))
}

fn save_config_for_state(
    state: &AppState,
    mut config: AppConfig,
) -> Result<(AppConfig, RagIndexAction), ConfigError> {
    config.agent.validate().map_err(ConfigError::Validation)?;
    config.active_cave = None;

    let mut rag_action = RagIndexAction::None;
    let response = update_agent_config(state, |stored| {
        rag_action = rag_index_action(&stored.agent.rag, &config.agent.rag);
        *stored = config;
        Ok(())
    })?;
    Ok((response, rag_action))
}

fn save_sidebar_state_for_state(
    state: &AppState,
    sidebar: SidebarConfig,
    agent_panel: SidebarConfig,
) -> Result<(), ConfigError> {
    let response = {
        let mut config = state.lock_config();
        config.sidebar = sidebar;
        config.agent_panel = agent_panel;
        config.clone()
    };
    state.save_config_to_cave(&response)?;
    Ok(())
}

fn list_providers_for_state(state: &AppState) -> Vec<granit_types::ProviderInfo> {
    let config = state.lock_config();
    config
        .agent
        .providers
        .iter()
        .enumerate()
        .map(|(i, entry): (usize, _)| granit_types::ProviderInfo {
            index: i,
            display_name: entry.display_name(),
            provider_type: entry.provider.provider_type().to_string(),
        })
        .collect()
}

fn select_provider_for_state(state: &AppState, index: usize) -> Result<AppConfig, ConfigError> {
    update_agent_config(state, |config| {
        if index >= config.agent.providers.len() {
            return Err(ConfigError::Validation(format!(
                "Provider index {index} out of range"
            )));
        }
        config.agent.selected_provider = index;
        config.agent.selected_model = None;
        Ok(())
    })
}

fn current_provider_for_model_listing(state: &AppState) -> Result<ProviderConfig, AgentError> {
    let config = state.lock_config();
    if config.agent.providers.is_empty() {
        return Err(AgentError::NoProviders);
    }
    let entry = config
        .agent
        .providers
        .get(config.agent.selected_provider)
        .ok_or(AgentError::ProviderIndexOutOfRange(
            config.agent.selected_provider,
        ))?;
    Ok(entry.provider.clone())
}

fn select_model_for_state(state: &AppState, model_id: String) -> Result<AppConfig, ConfigError> {
    update_agent_config(state, |config| {
        config.agent.selected_model = Some(model_id);
        Ok(())
    })
}

#[tauri::command]
pub(crate) fn get_config(state: tauri::State<AppState>) -> Result<AppConfig, ConfigError> {
    Ok(get_config_for_state(state.inner()))
}

#[tauri::command]
pub(crate) fn get_app_metadata() -> AppMetadata {
    AppMetadata::new(
        option_env!("GRANIT_GIT_HASH").unwrap_or("unknown"),
        option_env!("GRANIT_GIT_DIRTY").unwrap_or("false") == "true",
    )
}

#[tauri::command]
pub(crate) fn list_system_fonts() -> Vec<String> {
    let source = font_kit::source::SystemSource::new();
    let mut families = source.all_families().unwrap_or_default();
    families.sort();
    families.dedup();
    families
}

#[tauri::command]
pub(crate) fn save_config(
    config: AppConfig,
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<AppConfig, ConfigError> {
    let (response, rag_action) = save_config_for_state(state.inner(), config)?;

    // Apply the index consequence of the RAG settings change. Without this,
    // enabling RAG or switching the embedding model would silently keep
    // serving the old (or no) index until the cave is reopened.
    match rag_action {
        RagIndexAction::Rebuild => {
            if state.active_cave_path().is_some() {
                // Use the config this save just persisted — re-locking here
                // could pick up a concurrent save's settings and rebuild with
                // the wrong model (or after that save disabled RAG).
                build_vector_index(
                    state.core(),
                    &response.agent.rag,
                    &embedding_model_dirs(&app),
                    tauri::async_runtime::handle().inner(),
                );
            }
        }
        RagIndexAction::Disable => state.set_vector_index(None),
        RagIndexAction::None => {}
    }

    Ok(response)
}

#[tauri::command]
pub(crate) fn save_sidebar_state(
    sidebar: SidebarConfig,
    agent_panel: SidebarConfig,
    state: tauri::State<AppState>,
) -> Result<(), ConfigError> {
    save_sidebar_state_for_state(state.inner(), sidebar, agent_panel)
}

/// Where the RAG embedding model is cached and bundled, from the Tauri path
/// API: downloads go to `<app data>/models`, the bundled default model sits
/// in the app's resources.
fn embedding_model_dirs(app: &impl Manager<tauri::Wry>) -> EmbeddingModelDirs {
    let models_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("models");
    let bundled_model_dir = app
        .path()
        .resource_dir()
        .ok()
        .map(|d| d.join("resources/models/all-MiniLM-L6-v2-onnx"));
    EmbeddingModelDirs {
        models_dir,
        bundled_model_dir,
    }
}

/// The full open-cave flow, shared by the `open_cave` command and restore
/// (which reopens the restored directory the same way).
pub(crate) fn open_cave_at(
    path: PathBuf,
    app: &tauri::AppHandle,
    state: &AppState,
) -> Result<AppConfig, CaveError> {
    let prepared = prepare_cave(path.clone())?;

    // The presentation window belongs to the cave being closed.
    super::close_presentation_window(app, state);

    let store = Store::new(app);
    store.persist_active_cave(&path).map_err(CaveError::Io)?;

    state.install_cave(prepared);
    // The vector index is built on the tokio runtime behind
    // `tauri::async_runtime`.
    sync_vector_index(
        state.core(),
        &embedding_model_dirs(app),
        tauri::async_runtime::handle().inner(),
    );

    let config = state.lock_config();
    Ok(state.ipc_response(&config))
}

#[tauri::command]
pub(crate) fn open_cave(
    path: PathBuf,
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<AppConfig, CaveError> {
    open_cave_at(path, &app, &state)
}

#[tauri::command]
pub(crate) fn list_providers(
    state: tauri::State<AppState>,
) -> Result<Vec<granit_types::ProviderInfo>, ConfigError> {
    Ok(list_providers_for_state(state.inner()))
}

#[tauri::command]
pub(crate) fn select_provider(
    index: usize,
    state: tauri::State<AppState>,
) -> Result<AppConfig, ConfigError> {
    select_provider_for_state(state.inner(), index)
}

#[tauri::command]
pub(crate) async fn list_models(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ModelInfo>, AgentError> {
    let provider = current_provider_for_model_listing(state.inner())?;
    agent::list_models(&provider).await
}

#[tauri::command]
pub(crate) fn select_model(
    model_id: String,
    state: tauri::State<AppState>,
) -> Result<AppConfig, ConfigError> {
    select_model_for_state(state.inner(), model_id)
}

#[tauri::command]
pub(crate) fn select_mode(
    mode: granit_types::AgentMode,
    state: tauri::State<AppState>,
) -> Result<AppConfig, ConfigError> {
    update_agent_config(state.inner(), |config| {
        config.agent.mode = mode;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cave::Cave;
    use granit_types::{AgentConfig, AgentMode, RagConfig, ToolsConfig};

    fn test_app_state() -> AppState {
        AppState::new(AppConfig::default())
    }

    fn test_app_state_with_cave() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        cave.ensure_config().unwrap();

        let state = test_app_state();
        state.set_cave(Some(cave));
        (dir, state)
    }

    fn provider_entry(name: Option<&str>, provider: ProviderConfig) -> granit_types::ProviderEntry {
        granit_types::ProviderEntry {
            name: name.map(str::to_string),
            provider,
        }
    }

    // ── restore_cave_or_empty: startup must never fail ──────────────────────
    //
    // Regression guard for the crash where the setup hook propagated a
    // cave-open error. tao runs the setup hook in a non-unwinding context, so a
    // propagated `Err` aborts the whole app. `restore_cave_or_empty` must always
    // degrade to a non-fatal outcome instead.

    #[test]
    fn restore_cave_or_empty_restores_a_valid_cave() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("note.md"), "# hi").unwrap();
        let state = test_app_state();

        let outcome = restore_cave_or_empty(Some(dir.path().to_path_buf()), &state);

        assert_eq!(outcome, RestoreOutcome::Restored);
        assert_eq!(state.active_cave_path(), Some(dir.path().to_path_buf()));
    }

    #[test]
    fn restore_cave_or_empty_is_non_fatal_for_a_broken_cave() {
        // Two notes share a slug across subdirectories, so `Cave::open` returns
        // `DuplicateSlug`. This stands in for any open failure (incl. the
        // macOS `EPERM` that originally crashed the app): the error path is
        // identical, and unlike permission tricks it is portable to CI.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("dup.md"), "a").unwrap();
        std::fs::write(dir.path().join("sub/dup.md"), "b").unwrap();
        let state = test_app_state();

        // Must NOT propagate (would abort the setup hook); degrades to Failed.
        let outcome = restore_cave_or_empty(Some(dir.path().to_path_buf()), &state);

        assert_eq!(outcome, RestoreOutcome::Failed);
        assert!(
            state.active_cave_path().is_none(),
            "no cave should be set when restore fails"
        );
    }

    #[test]
    fn restore_cave_or_empty_is_non_fatal_for_a_duplicate_anchor_cave() {
        // The hard heading-anchor uniqueness check makes `Cave::open` fail;
        // restoring such a cave at startup must not crash the app either.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "# X {#dup}\n").unwrap();
        std::fs::write(dir.path().join("b.md"), "# Y {#dup}\n").unwrap();
        let state = test_app_state();

        let outcome = restore_cave_or_empty(Some(dir.path().to_path_buf()), &state);

        assert_eq!(outcome, RestoreOutcome::Failed);
        assert!(state.active_cave_path().is_none());
    }

    #[test]
    fn restore_cave_or_empty_reports_invalid_path() {
        let state = test_app_state();
        let missing = PathBuf::from("/granit/definitely/does/not/exist");

        assert_eq!(
            restore_cave_or_empty(Some(missing), &state),
            RestoreOutcome::InvalidPath
        );
        assert!(state.active_cave_path().is_none());
    }

    // ── ensure_system_prompt_file: seeding + legacy migration ──────────────

    #[test]
    fn system_prompt_file_is_seeded_with_default_on_restore() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state();

        let outcome = restore_cave_or_empty(Some(dir.path().to_path_buf()), &state);

        assert_eq!(outcome, RestoreOutcome::Restored);
        let seeded = std::fs::read_to_string(dir.path().join(".granit/agent/system.md")).unwrap();
        assert_eq!(seeded, granit_types::DEFAULT_SYSTEM_PROMPT_TEMPLATE);
    }

    // ── ensure_presentation_defaults: seeding on open ──────────────────────

    #[test]
    fn presentation_defaults_are_seeded_on_restore_only_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state();

        assert_eq!(
            restore_cave_or_empty(Some(dir.path().to_path_buf()), &state),
            RestoreOutcome::Restored
        );
        let presentations = dir.path().join(".granit/presentations");
        assert!(presentations.join("default-dark.css").exists());
        assert!(presentations.join("default-light.css").exists());
        let listed = state.with_cave(|cave| cave.list_presentations()).unwrap();
        assert_eq!(listed.len(), 2);

        // A deleted default stays deleted across reopen.
        std::fs::remove_file(presentations.join("default-light.css")).unwrap();
        let state = test_app_state();
        assert_eq!(
            restore_cave_or_empty(Some(dir.path().to_path_buf()), &state),
            RestoreOutcome::Restored
        );
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

        let state = test_app_state();
        let outcome = restore_cave_or_empty(Some(dir.path().to_path_buf()), &state);
        assert_eq!(outcome, RestoreOutcome::Restored);

        // The legacy value seeds the file and is cleared from the config,
        // in memory and on disk.
        let seeded = std::fs::read_to_string(dir.path().join(".granit/agent/system.md")).unwrap();
        assert_eq!(seeded, "my custom prompt");
        assert!(state.lock_config().agent.system_prompt.is_none());
        let saved = cave.load_config().unwrap();
        assert!(saved.agent.system_prompt.is_none());
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

        let state = test_app_state();
        restore_cave_or_empty(Some(dir.path().to_path_buf()), &state);

        let content = std::fs::read_to_string(dir.path().join(".granit/agent/system.md")).unwrap();
        assert_eq!(content, "user-edited prompt");
        assert!(state.lock_config().agent.system_prompt.is_none());
    }

    #[test]
    fn system_prompt_seeding_is_idempotent_across_reopens() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state();

        restore_cave_or_empty(Some(dir.path().to_path_buf()), &state);
        let path = dir.path().join(".granit/agent/system.md");
        std::fs::write(&path, "edited by user").unwrap();

        restore_cave_or_empty(Some(dir.path().to_path_buf()), &state);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "edited by user");
    }

    #[test]
    fn restore_cave_or_empty_reports_nothing_stored() {
        let state = test_app_state();
        assert_eq!(
            restore_cave_or_empty(None, &state),
            RestoreOutcome::NothingStored
        );
    }

    fn multi_provider_agent_config() -> AgentConfig {
        AgentConfig {
            providers: vec![
                provider_entry(
                    Some("Local"),
                    ProviderConfig::Ollama {
                        base_url: Some("http://localhost:11434".into()),
                    },
                ),
                provider_entry(
                    None,
                    ProviderConfig::Anthropic {
                        api_key: "test-key".into(),
                    },
                ),
            ],
            selected_provider: 0,
            selected_model: Some("qwen3.5:9b".into()),
            mode: AgentMode::default(),
            max_history: 100,
            max_turns: 10,
            system_prompt: None,
            disabled_tools: Vec::new(),
            tool_config: ToolsConfig::default(),
            rag: RagConfig::default(),
        }
    }

    #[test]
    fn test_get_config_for_state_injects_active_cave_path() {
        let (dir, state) = test_app_state_with_cave();
        state.lock_config().active_cave = Some("/stale/path".into());

        let config = get_config_for_state(&state);

        assert_eq!(
            config.active_cave,
            Some(dir.path().to_string_lossy().into_owned())
        );
    }

    #[test]
    fn test_save_config_for_state_persists_config_and_resets_agent_generation() {
        let (dir, state) = test_app_state_with_cave();
        let mut config = AppConfig {
            theme: "latte".into(),
            ..AppConfig::default()
        };
        config.active_cave = Some("/should/not/persist".into());

        let (response, _) = save_config_for_state(&state, config).unwrap();

        assert_eq!(state.lock_config().theme, "latte");
        assert!(state.lock_config().active_cave.is_none());
        assert_eq!(
            response.active_cave,
            Some(dir.path().to_string_lossy().into_owned())
        );
        assert_eq!(state.agent_generation(), 1);

        let saved = state.lock_cave().as_ref().unwrap().load_config().unwrap();
        assert_eq!(saved.theme, "latte");
        assert!(saved.active_cave.is_none());
    }

    // rag_index_action itself is tested in granit_core::config.

    #[test]
    fn test_save_config_for_state_reports_rag_action() {
        let (_dir, state) = test_app_state_with_cave();

        // Default config has RAG enabled; disabling it must report Disable.
        let mut config = AppConfig::default();
        config.agent.rag.enabled = false;

        let (_, action) = save_config_for_state(&state, config).unwrap();
        assert_eq!(action, RagIndexAction::Disable);

        // Re-enabling reports Rebuild.
        let (_, action) = save_config_for_state(&state, AppConfig::default()).unwrap();
        assert_eq!(action, RagIndexAction::Rebuild);
    }

    #[test]
    fn test_save_sidebar_state_for_state_updates_memory_without_open_cave() {
        let state = test_app_state();
        let sidebar = SidebarConfig {
            visible: false,
            width: 288,
        };
        let agent_panel = SidebarConfig {
            visible: true,
            width: 384,
        };

        save_sidebar_state_for_state(&state, sidebar.clone(), agent_panel.clone()).unwrap();

        let config = state.lock_config();
        assert_eq!(config.sidebar, sidebar);
        assert_eq!(config.agent_panel, agent_panel);
    }

    #[test]
    fn test_save_sidebar_state_for_state_persists_to_open_cave() {
        let (_dir, state) = test_app_state_with_cave();
        let sidebar = SidebarConfig {
            visible: false,
            width: 300,
        };
        let agent_panel = SidebarConfig {
            visible: true,
            width: 360,
        };

        save_sidebar_state_for_state(&state, sidebar.clone(), agent_panel.clone()).unwrap();

        let saved = state.lock_cave().as_ref().unwrap().load_config().unwrap();
        assert_eq!(saved.sidebar, sidebar);
        assert_eq!(saved.agent_panel, agent_panel);
    }

    #[test]
    fn test_list_providers_for_state_returns_display_names_and_types() {
        let state = test_app_state();
        state.lock_config().agent = multi_provider_agent_config();

        let providers = list_providers_for_state(&state);

        assert_eq!(providers.len(), 2);
        assert_eq!(providers[0].index, 0);
        assert_eq!(providers[0].display_name, "Local");
        assert_eq!(providers[0].provider_type, "ollama");
        assert_eq!(providers[1].display_name, "Anthropic");
        assert_eq!(providers[1].provider_type, "anthropic");
    }

    #[test]
    fn test_select_provider_for_state_clears_selected_model_and_persists() {
        let (dir, state) = test_app_state_with_cave();
        state.lock_config().agent = multi_provider_agent_config();

        let response = select_provider_for_state(&state, 1).unwrap();

        assert_eq!(state.lock_config().agent.selected_provider, 1);
        assert_eq!(state.lock_config().agent.selected_model, None);
        assert_eq!(state.agent_generation(), 1);
        assert_eq!(
            response.active_cave,
            Some(dir.path().to_string_lossy().into_owned())
        );

        let saved = state.lock_cave().as_ref().unwrap().load_config().unwrap();
        assert_eq!(saved.agent.selected_provider, 1);
        assert_eq!(saved.agent.selected_model, None);
    }

    #[test]
    fn test_select_provider_for_state_rejects_out_of_range_index() {
        let (_dir, state) = test_app_state_with_cave();
        state.lock_config().agent = multi_provider_agent_config();

        let err = select_provider_for_state(&state, 7).unwrap_err();

        assert!(matches!(err, ConfigError::Validation(_)));
        assert_eq!(state.lock_config().agent.selected_provider, 0);
        assert_eq!(
            state.lock_config().agent.selected_model.as_deref(),
            Some("qwen3.5:9b")
        );
        assert_eq!(state.agent_generation(), 0);
    }

    #[test]
    fn test_current_provider_for_model_listing_errors_when_no_providers_exist() {
        let state = test_app_state();
        state.lock_config().agent.providers.clear();

        let err = current_provider_for_model_listing(&state).unwrap_err();

        assert!(matches!(err, AgentError::NoProviders));
    }

    #[test]
    fn test_current_provider_for_model_listing_errors_for_out_of_range_provider() {
        let state = test_app_state();
        let mut agent = multi_provider_agent_config();
        agent.selected_provider = 5;
        state.lock_config().agent = agent;

        let err = current_provider_for_model_listing(&state).unwrap_err();

        assert!(matches!(err, AgentError::ProviderIndexOutOfRange(5)));
    }

    #[test]
    fn test_select_model_for_state_updates_selection_and_resets_agent_generation() {
        let (dir, state) = test_app_state_with_cave();

        let response = select_model_for_state(&state, "claude-sonnet-4-6".into()).unwrap();

        assert_eq!(
            state.lock_config().agent.selected_model.as_deref(),
            Some("claude-sonnet-4-6")
        );
        assert_eq!(state.agent_generation(), 1);
        assert_eq!(
            response.active_cave,
            Some(dir.path().to_string_lossy().into_owned())
        );

        let saved = state.lock_cave().as_ref().unwrap().load_config().unwrap();
        assert_eq!(
            saved.agent.selected_model.as_deref(),
            Some("claude-sonnet-4-6")
        );
    }

    #[test]
    fn test_restore_cave_logic_no_stored_key_leaves_state_unchanged() {
        let state = test_app_state();

        let outcome = restore_cave_logic(None, &state).unwrap();

        assert_eq!(outcome, RestoreOutcome::NothingStored);
        assert!(state.lock_cave().is_none());
    }

    #[test]
    fn test_restore_cave_logic_non_directory_path_returns_invalid_path() {
        let state = test_app_state();
        let path = PathBuf::from("/this/path/does/not/exist");

        let outcome = restore_cave_logic(Some(path), &state).unwrap();

        assert_eq!(outcome, RestoreOutcome::InvalidPath);
        assert!(state.lock_cave().is_none());
    }

    #[test]
    fn test_restore_cave_logic_valid_dir_loads_cave_and_config_into_state() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state();

        // Write a custom config to the cave so we can verify it gets loaded.
        let cave = Cave::open(dir.path().to_path_buf()).unwrap();
        cave.ensure_config().unwrap();
        let mut expected_config = cave.load_config().unwrap();
        expected_config.theme = "dracula".into();
        cave.save_config(&expected_config).unwrap();

        let outcome = restore_cave_logic(Some(dir.path().to_path_buf()), &state).unwrap();

        assert_eq!(outcome, RestoreOutcome::Restored);
        assert_eq!(state.active_cave_path(), Some(dir.path().to_path_buf()));
        assert_eq!(state.lock_config().theme, "dracula");
    }
}

use super::store::Store;
use super::AppState;
use crate::agent::{self, AgentError};
use crate::cave::{Cave, CaveError};
use granit_types::{AppConfig, AppMetadata, ModelInfo, ProviderConfig, RagConfig, SidebarConfig};
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

    let cave = Cave::open(path)?;
    cave.ensure_config()?;
    let config = cave.load_config()?;
    *state.lock_config() = config;
    state.set_cave(Some(cave));

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
        let rag_config = state.lock_config().agent.rag.clone();
        if rag_config.enabled {
            build_vector_index(app, &state, &rag_config);
        }
    }

    Ok(())
}

fn get_config_for_state(state: &AppState) -> AppConfig {
    let config = state.lock_config();
    state.ipc_response(&config)
}

fn save_config_for_state(
    state: &AppState,
    mut config: AppConfig,
) -> Result<AppConfig, ConfigError> {
    config.agent.validate().map_err(ConfigError::Validation)?;

    let response = {
        config.active_cave = None;

        let mut stored_config = state.lock_config();
        *stored_config = config.clone();
        config
    };

    state.save_config_to_cave(&response)?;
    state.reset_agent();
    Ok(state.ipc_response(&response))
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
    let response = {
        let mut config = state.lock_config();
        if index >= config.agent.providers.len() {
            return Err(ConfigError::Validation(format!(
                "Provider index {index} out of range"
            )));
        }
        config.agent.selected_provider = index;
        config.agent.selected_model = None;
        config.clone()
    };

    state.save_config_to_cave(&response)?;
    state.reset_agent();
    Ok(state.ipc_response(&response))
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
    let response = {
        let mut config = state.lock_config();
        config.agent.selected_model = Some(model_id);
        config.clone()
    };

    state.save_config_to_cave(&response)?;
    state.reset_agent();
    Ok(state.ipc_response(&response))
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
    state: tauri::State<AppState>,
) -> Result<AppConfig, ConfigError> {
    save_config_for_state(state.inner(), config)
}

#[tauri::command]
pub(crate) fn save_sidebar_state(
    sidebar: SidebarConfig,
    agent_panel: SidebarConfig,
    state: tauri::State<AppState>,
) -> Result<(), ConfigError> {
    save_sidebar_state_for_state(state.inner(), sidebar, agent_panel)
}

/// Resolve paths and build the vector index for RAG, spawning the async
/// rebuild in the background.
fn build_vector_index(app: &impl Manager<tauri::Wry>, state: &AppState, rag_config: &RagConfig) {
    let models_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("models");
    let _ = std::fs::create_dir_all(&models_dir);

    let model_name =
        crate::agent::vectordb::resolve_model_name(rag_config.embedding_model.as_deref());

    let bundled_model_dir = app
        .path()
        .resource_dir()
        .ok()
        .map(|d| d.join("resources/models/all-MiniLM-L6-v2-onnx"))
        .filter(|d| d.join("model.onnx").exists());

    match crate::agent::vectordb::CaveVectorIndex::new(
        state.shared_cave(),
        model_name,
        &models_dir,
        bundled_model_dir.as_deref(),
    ) {
        Ok(index) => {
            state.set_vector_index(Some(index.clone()));
            tauri::async_runtime::spawn(async move {
                if let Err(e) = index.rebuild().await {
                    warn!("vector index rebuild failed: {e}");
                }
            });
        }
        Err(e) => {
            warn!("vector index init failed (RAG disabled): {e}");
            state.set_vector_index(None);
        }
    }
}

#[tauri::command]
pub(crate) fn open_cave(
    path: PathBuf,
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> Result<AppConfig, CaveError> {
    let cave = Cave::open(path.clone())?;
    cave.ensure_config()?;
    let config = cave.load_config()?;

    let store = Store::new(&app);
    store.persist_active_cave(&path).map_err(CaveError::Io)?;

    *state.lock_config() = config;
    state.set_cave(Some(cave));
    state.reset_agent();

    let rag_config = state.lock_config().agent.rag.clone();
    if rag_config.enabled {
        build_vector_index(&app, &state, &rag_config);
    } else {
        state.set_vector_index(None);
    }

    let config = state.lock_config();
    Ok(state.ipc_response(&config))
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
    let response = {
        let mut config = state.lock_config();
        config.agent.mode = mode;
        config.clone()
    };
    state.save_config_to_cave(&response)?;
    state.reset_agent();
    Ok(state.ipc_response(&response))
}

#[cfg(test)]
mod tests {
    use super::*;
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

        let response = save_config_for_state(&state, config).unwrap();

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

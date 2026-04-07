mod agent;
mod cave;
mod config;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::agent::{Agent, AgentError, SharedCave};
use crate::cave::{Cave, CaveError};
use granit_types::AppConfig;

pub(crate) use agent::{clear_chat, list_tools, send_message};
pub(crate) use cave::{
    create_folder, create_note, create_template, delete_folder, delete_note, delete_template,
    list_folders, list_notes, list_templates, list_todos, move_folder, move_note,
    open_daily_note, read_note, read_template, rename_folder, rename_note, rename_template,
    render_markdown, render_note, render_template, save_note, save_template, search_content,
    set_active_note, toggle_todo, toggle_todo_by_index, update_note, update_template,
};
pub(crate) use config::{
    get_app_metadata, get_config, list_models, list_providers, list_system_fonts, open_cave,
    restore_active_cave, save_config, save_sidebar_state, select_model, select_provider,
};

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

pub(crate) struct AppState {
    config: Mutex<AppConfig>,
    cave: SharedCave,
    agent: Mutex<Option<Agent>>,
    agent_generation: AtomicU64,
}

impl AppState {
    pub(crate) fn new(config: AppConfig) -> Self {
        Self {
            config: Mutex::new(config),
            cave: Arc::new(Mutex::new(None)),
            agent: Mutex::new(None),
            agent_generation: AtomicU64::new(0),
        }
    }

    pub(super) fn lock_config(&self) -> std::sync::MutexGuard<'_, AppConfig> {
        self.config.lock().expect("config mutex poisoned")
    }

    pub(super) fn lock_cave(&self) -> std::sync::MutexGuard<'_, Option<Cave>> {
        self.cave.lock().expect("cave mutex poisoned")
    }

    pub(super) fn lock_agent(&self) -> std::sync::MutexGuard<'_, Option<Agent>> {
        self.agent.lock().expect("agent mutex poisoned")
    }

    pub(super) fn active_cave_path(&self) -> Option<std::path::PathBuf> {
        self.lock_cave().as_ref().map(|c| c.path().to_path_buf())
    }

    pub(super) fn set_cave(&self, cave: Option<Cave>) {
        *self.lock_cave() = cave;
    }

    pub(super) fn reset_agent(&self) {
        self.agent_generation.fetch_add(1, Ordering::Relaxed);
        *self.lock_agent() = None;
    }

    pub(super) fn agent_generation(&self) -> u64 {
        self.agent_generation.load(Ordering::Relaxed)
    }

    pub(super) fn ensure_agent(&self) -> Result<(), AgentError> {
        if self.lock_agent().is_some() {
            return Ok(());
        }
        let agent_config = self.lock_config().agent.clone();
        *self.lock_agent() = Some(Agent::from_config(&agent_config, self.cave.clone())?);
        Ok(())
    }

    pub(super) fn ipc_response(&self, config: &AppConfig) -> AppConfig {
        let mut ipc = config.clone();
        ipc.active_cave = self
            .active_cave_path()
            .map(|p| p.to_string_lossy().into_owned());
        ipc
    }
}

pub(super) fn save_config_to_active_cave(
    state: &AppState,
    config: &AppConfig,
) -> Result<(), ConfigError> {
    let cave = state.lock_cave();
    let cave = cave
        .as_ref()
        .ok_or_else(|| ConfigError::Validation("No cave is currently open".to_string()))?;
    cave.save_config(config)
        .map_err(|err| ConfigError::Validation(err.to_string()))
}

pub(super) fn save_config_to_active_cave_if_open(
    state: &AppState,
    config: &AppConfig,
) -> Result<(), ConfigError> {
    let cave = state.lock_cave();
    let Some(cave) = cave.as_ref() else {
        return Ok(());
    };
    cave.save_config(config)
        .map_err(|err| ConfigError::Validation(err.to_string()))
}

pub(super) fn with_cave<F, T>(state: &tauri::State<AppState>, f: F) -> Result<T, CaveError>
where
    F: FnOnce(&Cave) -> Result<T, CaveError>,
{
    let guard = state.lock_cave();
    let cave = guard.as_ref().ok_or(CaveError::NoCaveOpen)?;
    f(cave)
}

pub(super) fn with_cave_mut<F, T>(state: &tauri::State<AppState>, f: F) -> Result<T, CaveError>
where
    F: FnOnce(&mut Cave) -> Result<T, CaveError>,
{
    let mut guard = state.lock_cave();
    let cave = guard.as_mut().ok_or(CaveError::NoCaveOpen)?;
    f(cave)
}
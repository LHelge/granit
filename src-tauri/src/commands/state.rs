use std::ops::Deref;

use parking_lot::{Mutex, MutexGuard};

use granit_core::CoreState;
use granit_types::AppConfig;

use super::{ConfigError, ShownPresentation};

/// The `tauri::manage`d app state: the host-agnostic [`CoreState`] (config,
/// cave, agent, vector index and their locking helpers, reached through
/// `Deref`) plus what only the Tauri app has, the presentation window.
pub(crate) struct AppState {
    core: CoreState,
    /// What the presentation window shows, while it is open.
    presentation: Mutex<Option<ShownPresentation>>,
}

impl Deref for AppState {
    type Target = CoreState;

    fn deref(&self) -> &CoreState {
        &self.core
    }
}

impl AppState {
    pub(crate) fn new(config: AppConfig) -> Self {
        Self {
            core: CoreState::new(config),
            presentation: Mutex::new(None),
        }
    }

    pub(super) fn lock_presentation(&self) -> MutexGuard<'_, Option<ShownPresentation>> {
        self.presentation.lock()
    }

    pub(super) fn ipc_response(&self, config: &AppConfig) -> AppConfig {
        let mut ipc = config.clone();
        ipc.active_cave = self
            .active_cave_path()
            .map(|p| p.to_string_lossy().into_owned());
        ipc
    }

    pub(super) fn save_config_to_cave(&self, config: &AppConfig) -> Result<(), ConfigError> {
        let cave = self.lock_cave();
        let Some(cave) = cave.as_ref() else {
            return Ok(());
        };
        cave.save_config(config)
            .map_err(|err| ConfigError::Validation(err.to_string()))
    }
}

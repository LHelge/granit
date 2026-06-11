use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::{Mutex, MutexGuard};

use crate::agent::vectordb::CaveVectorIndex;
use crate::agent::{Agent, AgentError};
use crate::cave::{Cave, CaveError};
use granit_types::AppConfig;

use super::ConfigError;

/// Shared handle to the currently open cave, used by AppState and agent tools.
pub type SharedCave = Arc<Mutex<Option<Cave>>>;

/// Lock a `SharedCave`, ensure it is scanned, and run a closure on it.
///
/// This is the single implementation used by both `AppState::with_cave`
/// and the agent tool helpers.
pub(crate) fn with_shared_cave<F, T>(cave: &SharedCave, f: F) -> Result<T, CaveError>
where
    F: FnOnce(&mut Cave) -> Result<T, CaveError>,
{
    let mut guard = cave.lock();
    let cave = guard.as_mut().ok_or(CaveError::NoCaveOpen)?;
    cave.ensure_scanned()?;
    f(cave)
}

pub(crate) struct AppState {
    config: Mutex<AppConfig>,
    cave: SharedCave,
    agent: Mutex<Option<Agent>>,
    agent_generation: AtomicU64,
    vector_index: Mutex<Option<CaveVectorIndex>>,
}

impl AppState {
    pub(crate) fn new(config: AppConfig) -> Self {
        Self {
            config: Mutex::new(config),
            cave: Arc::new(Mutex::new(None)),
            agent: Mutex::new(None),
            agent_generation: AtomicU64::new(0),
            vector_index: Mutex::new(None),
        }
    }

    pub(super) fn lock_config(&self) -> MutexGuard<'_, AppConfig> {
        self.config.lock()
    }

    pub(super) fn lock_cave(&self) -> MutexGuard<'_, Option<Cave>> {
        self.cave.lock()
    }

    pub(super) fn shared_cave(&self) -> SharedCave {
        self.cave.clone()
    }

    pub(super) fn lock_agent(&self) -> MutexGuard<'_, Option<Agent>> {
        self.agent.lock()
    }

    pub(super) fn active_cave_path(&self) -> Option<std::path::PathBuf> {
        self.lock_cave().as_ref().map(|c| c.path().to_path_buf())
    }

    pub(super) fn set_cave(&self, cave: Option<Cave>) {
        *self.lock_cave() = cave;
    }

    pub(super) fn set_vector_index(&self, index: Option<CaveVectorIndex>) {
        let old = {
            let mut guard = self.vector_index.lock();
            std::mem::replace(&mut *guard, index)
        };
        // Abort any in-flight rebuild on the superseded index so it stops
        // embedding and never overwrites the new index's cache file.
        if let Some(old) = old {
            old.cancel();
        }
    }

    pub(super) fn vector_index(&self) -> Option<CaveVectorIndex> {
        self.vector_index.lock().clone()
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
        let vector_index = self.vector_index();
        *self.lock_agent() = Some(Agent::from_config(
            &agent_config,
            self.cave.clone(),
            vector_index,
        )?);
        Ok(())
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

    pub(super) fn with_cave<F, T>(&self, f: F) -> Result<T, CaveError>
    where
        F: FnOnce(&mut Cave) -> Result<T, CaveError>,
    {
        with_shared_cave(&self.cave, f)
    }
}

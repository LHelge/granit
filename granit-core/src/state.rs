use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::{Mutex, MutexGuard};

use crate::agent::vectordb::CaveVectorIndex;
use crate::agent::{Agent, AgentError};
use crate::cave::{Cave, CaveError};
use granit_types::AppConfig;

/// Shared handle to the currently open cave, used by `CoreState` and agent tools.
pub type SharedCave = Arc<Mutex<Option<Cave>>>;

/// Lock a `SharedCave`, ensure it is scanned, and run a closure on it.
///
/// This is the single implementation used by both `CoreState::with_cave`
/// and the agent tool helpers.
pub fn with_shared_cave<F, T>(cave: &SharedCave, f: F) -> Result<T, CaveError>
where
    F: FnOnce(&mut Cave) -> Result<T, CaveError>,
{
    let mut guard = cave.lock();
    let cave = guard.as_mut().ok_or(CaveError::NoCaveOpen)?;
    cave.ensure_scanned()?;
    f(cave)
}

/// The host-agnostic state of a running Granit app: the cave config, the open
/// cave (which also holds the active note), the lazily built agent with its
/// generation counter, and the RAG vector index — each behind its own mutex.
///
/// A UI host keeps one of these for the app's lifetime and adds its own
/// window, event and plugin state around it.
pub struct CoreState {
    config: Mutex<AppConfig>,
    cave: SharedCave,
    agent: Mutex<Option<Agent>>,
    agent_generation: AtomicU64,
    vector_index: Mutex<Option<CaveVectorIndex>>,
}

impl CoreState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: Mutex::new(config),
            cave: Arc::new(Mutex::new(None)),
            agent: Mutex::new(None),
            agent_generation: AtomicU64::new(0),
            vector_index: Mutex::new(None),
        }
    }

    pub fn lock_config(&self) -> MutexGuard<'_, AppConfig> {
        self.config.lock()
    }

    pub fn lock_cave(&self) -> MutexGuard<'_, Option<Cave>> {
        self.cave.lock()
    }

    pub fn shared_cave(&self) -> SharedCave {
        self.cave.clone()
    }

    pub fn lock_agent(&self) -> MutexGuard<'_, Option<Agent>> {
        self.agent.lock()
    }

    pub fn active_cave_path(&self) -> Option<std::path::PathBuf> {
        self.lock_cave().as_ref().map(|c| c.path().to_path_buf())
    }

    pub fn set_cave(&self, cave: Option<Cave>) {
        *self.lock_cave() = cave;
    }

    pub fn set_vector_index(&self, index: Option<CaveVectorIndex>) {
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

    pub fn vector_index(&self) -> Option<CaveVectorIndex> {
        self.vector_index.lock().clone()
    }

    pub fn reset_agent(&self) {
        self.agent_generation.fetch_add(1, Ordering::Relaxed);
        *self.lock_agent() = None;
    }

    pub fn agent_generation(&self) -> u64 {
        self.agent_generation.load(Ordering::Relaxed)
    }

    pub fn ensure_agent(&self) -> Result<(), AgentError> {
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

    pub fn with_cave<F, T>(&self, f: F) -> Result<T, CaveError>
    where
        F: FnOnce(&mut Cave) -> Result<T, CaveError>,
    {
        with_shared_cave(&self.cave, f)
    }
}

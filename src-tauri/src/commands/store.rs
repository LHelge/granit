use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use tauri::{Manager, Runtime};
use tauri_plugin_store::StoreExt;

pub(super) struct Store<'a, R: Runtime, M: Manager<R>> {
    manager: &'a M,
    _runtime: PhantomData<R>,
}

impl<'a, R: Runtime, M: Manager<R>> Store<'a, R, M> {
    const APP_STATE_STORE_PATH: &'static str = "app-state.json";
    const ACTIVE_CAVE_STORE_KEY: &'static str = "active_cave";

    pub(super) fn new(manager: &'a M) -> Self {
        Self {
            manager,
            _runtime: PhantomData,
        }
    }

    pub(super) fn load_persisted_active_cave(&self) -> Result<Option<PathBuf>, String> {
        let store = self
            .manager
            .store(Self::APP_STATE_STORE_PATH)
            .map_err(|err| err.to_string())?;

        let Some(value) = store.get(Self::ACTIVE_CAVE_STORE_KEY) else {
            return Ok(None);
        };

        let Some(path) = value.as_str() else {
            store.delete(Self::ACTIVE_CAVE_STORE_KEY);
            store.save().map_err(|err| err.to_string())?;
            return Ok(None);
        };

        Ok(Some(PathBuf::from(path)))
    }

    pub(super) fn persist_active_cave(&self, path: &Path) -> Result<(), String> {
        let store = self
            .manager
            .store(Self::APP_STATE_STORE_PATH)
            .map_err(|err| err.to_string())?;
        store.set(
            Self::ACTIVE_CAVE_STORE_KEY,
            path.to_string_lossy().into_owned(),
        );
        store.save().map_err(|err| err.to_string())
    }

    pub(super) fn clear_persisted_active_cave(&self) -> Result<(), String> {
        let store = self
            .manager
            .store(Self::APP_STATE_STORE_PATH)
            .map_err(|err| err.to_string())?;
        store.delete(Self::ACTIVE_CAVE_STORE_KEY);
        store.save().map_err(|err| err.to_string())
    }
}
